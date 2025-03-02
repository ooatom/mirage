use super::*;
use crate::assets::{AssetHandle, Assets, Material};
use crate::gpu::GPU;
use crate::math::Mat4;
use crate::renderer::gpu_geom::GPUGeom;
use crate::renderer::gpu_texture::GPUTexture;
use crate::renderer::shading::Shading;
use crate::scene::vertex::Vertex;
use ash::vk;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{c_void, CStr};
use std::io;
use std::mem::{align_of, size_of};
use std::rc::Rc;

#[repr(C)]
#[derive(Copy, Clone, PartialEq)]
pub struct SceneData {
    pub view: Mat4,
    pub projection: Mat4,
    pub view_projection: Mat4,
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq)]
pub struct ObjectData {
    pub model: Mat4,
}

// https://stackoverflow.com/questions/28127165/how-to-convert-struct-to-u8
unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::core::slice::from_raw_parts((p as *const T) as *const u8, ::core::mem::size_of::<T>())
}

unsafe fn u8_slice_as_any<T>(p: &[u8]) -> &T {
    assert_eq!(p.len(), ::core::mem::size_of::<T>());
    &*(p.as_ptr() as *const T)
}

pub struct ForwardRenderer {
    gpu: Rc<GPU>,

    pub view: Mat4,
    pub projection: Mat4,
    pub pipeline_cache: RefCell<HashMap<u32, Pipeline>>,
    pub shading_cache: RefCell<HashMap<u32, Shading>>,

    pub geom_cache: RefCell<HashMap<u32, GPUGeom>>,
    pub texture_cache: RefCell<HashMap<u32, GPUTexture>>,

    pub descriptor_pool: vk::DescriptorPool,
    pub render_pass: vk::RenderPass,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_sets: Vec<vk::DescriptorSet>,

    pub depth_reverse_z: bool,

    framebuffers: Vec<vk::Framebuffer>,
    color_image: vk::Image,
    color_image_memory: vk::DeviceMemory,
    color_image_view: vk::ImageView,
    depth_image: vk::Image,
    depth_image_memory: vk::DeviceMemory,
    depth_image_view: vk::ImageView,

    uniform_buffers: Vec<vk::Buffer>,
    uniform_buffer_memories: Vec<vk::DeviceMemory>,
    uniform_buffer_memories_mapped: Vec<*mut c_void>,
}

impl ForwardRenderer {
    pub const FRAMES_IN_FLIGHT: u32 = 2;

    pub fn new(gpu: &Rc<GPU>) -> Self {
        unsafe {
            let descriptor_pool = Self::create_descriptor_pool(&gpu);
            let render_pass = Self::create_render_pass(gpu);
            let (color_image, color_image_memory, color_image_view) =
                Self::create_color_resources(gpu);
            let (depth_image, depth_image_memory, depth_image_view) =
                Self::create_depth_resources(gpu);
            let framebuffers =
                Self::create_framebuffers(gpu, render_pass, color_image_view, depth_image_view);

            let descriptor_set_layout =
                gpu.create_descriptor_set_layout(&vec![vk::DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    stage_flags: vk::ShaderStageFlags::ALL_GRAPHICS,
                    ..Default::default()
                }]);

            let descriptor_sets = gpu.create_descriptor_sets(
                descriptor_pool,
                &vec![descriptor_set_layout; Self::FRAMES_IN_FLIGHT as usize],
            );
            let (uniform_buffers, uniform_buffer_memories, uniform_buffer_memories_mapped) =
                Self::create_uniform_buffers(gpu);

            for (index, descriptor_set) in descriptor_sets.iter().enumerate() {
                let buffer_infos = [vk::DescriptorBufferInfo {
                    buffer: uniform_buffers[index],
                    offset: 0,
                    range: size_of::<SceneData>() as vk::DeviceSize,
                }];
                let ubo_write = vk::WriteDescriptorSet::default()
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(&buffer_infos)
                    .dst_set(*descriptor_set)
                    .dst_binding(0)
                    // starting element in that array
                    .dst_array_element(0);

                gpu.device_context
                    .device
                    .update_descriptor_sets(&[ubo_write], &[]);
            }

            Self {
                gpu: Rc::clone(gpu),
                shading_cache: RefCell::new(HashMap::new()),
                pipeline_cache: RefCell::new(HashMap::new()),
                geom_cache: RefCell::new(HashMap::new()),
                texture_cache: RefCell::new(HashMap::new()),

                view: Mat4::identity(),
                projection: Mat4::identity(),

                descriptor_pool,
                descriptor_set_layout,
                descriptor_sets,

                depth_reverse_z: false,

                framebuffers,
                render_pass,
                color_image,
                color_image_memory,
                color_image_view,
                depth_image,
                depth_image_memory,
                depth_image_view,

                uniform_buffers,
                uniform_buffer_memories,
                uniform_buffer_memories_mapped,
            }
        }
    }

    pub fn render(
        &self,
        command_buffer: vk::CommandBuffer,
        context: RenderContext,
        image_index: usize,
        frame_index: usize,
    ) {
        unsafe {
            let device = &self.gpu.device_context.device;
            let scene_data = SceneData {
                view: self.view,
                projection: self.projection,
                view_projection: self.projection * self.view,
            };
            let mut align = ash::util::Align::new(
                self.uniform_buffer_memories_mapped[frame_index],
                align_of::<SceneData>() as vk::DeviceSize,
                size_of::<SceneData>() as vk::DeviceSize,
            );
            align.copy_from_slice(&[scene_data]);

            let assets = context.assets.borrow();
            context.objects.iter().for_each(|object| {
                if let Err(msg) = self.preprocess_material(&context, &object.material) {
                    println!("preprocess_material failed, {}", msg);
                    return;
                }

                let shading_cache = self.shading_cache.borrow();
                let Some(shading) = shading_cache.get(&object.material.id) else {
                    return;
                };

                let material = assets.load(&object.material);
                let Some(texture) = &material.tex else {
                    return;
                };

                let mut texture_cache = self.texture_cache.borrow_mut();
                let texture = match texture_cache.get(&texture.id) {
                    None => {
                        let tex_asset = assets.load(&texture);
                        let tex_gpu = GPUTexture::new(&self.gpu, &tex_asset);
                        texture_cache.insert(texture.id, tex_gpu);
                        texture_cache.get(&texture.id).unwrap()
                    }
                    Some(geom) => geom,
                };

                let image_infos = [vk::DescriptorImageInfo {
                    image_view: texture.image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    sampler: texture.image_sampler,
                }];

                let texture_write = vk::WriteDescriptorSet::default()
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&image_infos)
                    .dst_set(shading.descriptor_sets[frame_index])
                    .dst_binding(0)
                    .dst_array_element(0);

                let sampler_write = vk::WriteDescriptorSet::default()
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .image_info(&image_infos)
                    .dst_set(shading.descriptor_sets[frame_index])
                    .dst_binding(1)
                    .dst_array_element(0);

                device.update_descriptor_sets(&[texture_write, sampler_write], &[]);
            });
        }

        unsafe {
            let device = &self.gpu.device_context.device;
            device.cmd_set_viewport(
                command_buffer,
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: self.gpu.swap_chain.extent.width as f32,
                    height: self.gpu.swap_chain.extent.height as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );
            device.cmd_set_scissor(
                command_buffer,
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.gpu.swap_chain.extent,
                }],
            );

            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: if self.depth_reverse_z { 0.0 } else { 1.0 },
                        stencil: 0,
                    },
                },
            ];

            let render_pass_begin_info = vk::RenderPassBeginInfo::default()
                .clear_values(&clear_values)
                .render_pass(self.render_pass)
                .framebuffer(self.framebuffers[image_index])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.gpu.swap_chain.extent,
                });

            // INLINE: The render pass commands will be embedded in the primary command buffer itself
            // and no secondary command buffers will be executed.
            // SECONDARY_COMMAND_BUFFERS: The render pass commands will be executed from secondary command buffers.
            device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );

            let assets = context.assets.borrow();
            context.objects.iter().for_each(|object| {
                let shading_cache = self.shading_cache.borrow();
                let Some(shading) = shading_cache.get(&object.material.id) else {
                    return;
                };
                let mut geom_cache = self.geom_cache.borrow_mut();
                let geom = match geom_cache.get(&object.geom.id) {
                    None => {
                        let geom_asset = assets.load(&object.geom);
                        let geom_gpu = GPUGeom::new(&self.gpu, &geom_asset);
                        geom_cache.insert(object.geom.id, geom_gpu);
                        geom_cache.get(&object.geom.id).unwrap()
                    }
                    Some(geom) => geom,
                };

                let object_data = ObjectData {
                    model: object.model,
                };
                device.cmd_push_constants(
                    command_buffer,
                    shading.pipeline.pipeline_layout,
                    vk::ShaderStageFlags::ALL_GRAPHICS,
                    0,
                    any_as_u8_slice(&object_data),
                );

                device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    shading.pipeline.pipeline_layout,
                    0,
                    &[
                        self.descriptor_sets[frame_index],
                        shading.descriptor_sets[frame_index],
                    ],
                    &[],
                );

                device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    shading.pipeline.pipeline,
                );

                device.cmd_bind_vertex_buffers(command_buffer, 0, &[geom.vertex_buffer], &[0]);
                device.cmd_bind_index_buffer(
                    command_buffer,
                    geom.index_buffer,
                    0,
                    vk::IndexType::UINT32,
                );
                // device.cmd_draw(command_buffer, );
                // device.cmd_draw_indexed(command_buffer, self.geom.indices.len() as u32, 1, 0, 0, 0);
                device.cmd_draw_indexed(command_buffer, geom.indices_length as u32, 1, 0, 0, 0);
            });

            device.cmd_end_render_pass(command_buffer);
        }
    }

    pub fn preprocess_material(
        &self,
        context: &RenderContext,
        material: &AssetHandle<Material>,
    ) -> Result<(), &str> {
        // The Vulkan SDK includes libshaderc, which is a library to compile GLSL code to SPIR-V from within your program.
        // https://github.com/google/shaderc
        // little endian
        // let mut buffer = Cursor::new(Shaders::get("simple.vert.spv").unwrap().data);
        // let vert_shader_code = ash::util::read_spv(&mut buffer).unwrap();
        // let mut buffer = Cursor::new(Shaders::get("simple.frag.spv").unwrap().data);
        // let frag_shader_code = ash::util::read_spv(&mut buffer).unwrap();

        // let vert_shader_module = device.create_shader_module(&vert_shader_code);
        // let frag_shader_module = device.create_shader_module(&frag_shader_code);
        let def = ShadingDef::load("simple.spv");
        if def.mode != ShadingMode::Unlit {
            return Err("Unsupported ShadingMode");
        }

        if let Some(_) = self.shading_cache.borrow_mut().get(&material.id) {
            return Ok(());
        }

        let mut pipeline_cache = self.pipeline_cache.borrow_mut();
        let pipeline = if let Some(&pipeline) = pipeline_cache.get(&def.id) {
            pipeline
        } else {
            let data = Assets::load_raw(def.path).unwrap();
            let mut buffer = io::Cursor::new(&data);
            let shader_code = ash::util::read_spv(&mut buffer).unwrap();
            let shader_module = self.gpu.create_shader_module(&shader_code);

            let descriptor_set_layout = self.gpu.create_descriptor_set_layout(&def.bindings);
            let (pipeline, pipeline_layout) =
                self.create_pipeline(&def, shader_module, descriptor_set_layout);

            let pipeline = Pipeline {
                shader_module,
                descriptor_set_layout,
                pipeline_layout,
                pipeline,
            };
            pipeline_cache.insert(def.id, pipeline);

            pipeline
        };

        let descriptor_sets = self.gpu.create_descriptor_sets(
            self.descriptor_pool,
            &vec![pipeline.descriptor_set_layout; Self::FRAMES_IN_FLIGHT as usize],
        );
        let shading = Shading {
            pipeline,
            pipeline_dirty: false,
            descriptor_sets,
        };

        self.shading_cache.borrow_mut().insert(material.id, shading);

        Ok(())
    }

    pub fn clear_cache(&mut self) {
        unsafe {
            let device = &self.gpu.device_context.device;

            self.pipeline_cache
                .get_mut()
                .iter()
                .for_each(|(_, pipeline)| {
                    device.destroy_descriptor_set_layout(pipeline.descriptor_set_layout, None);
                    device.destroy_shader_module(pipeline.shader_module, None);
                    device.destroy_pipeline(pipeline.pipeline, None);
                    device.destroy_pipeline_layout(pipeline.pipeline_layout, None);
                });
        }
    }

    fn create_pipeline(
        &self,
        def: &ShadingDef,
        shader_module: vk::ShaderModule,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> (vk::Pipeline, vk::PipelineLayout) {
        unsafe {
            let vert_shader_stage = vk::PipelineShaderStageCreateInfo::default()
                .module(shader_module)
                .stage(vk::ShaderStageFlags::VERTEX)
                .name(CStr::from_bytes_with_nul_unchecked(b"vs\0"));
            // It allows you to specify values for shader constants. You can use a single shader module where its behavior can be configured
            // at pipeline creation by specifying different values for the constants used in it. This is more efficient than configuring
            // the shader using variables at render time, because the compiler can do optimizations like eliminating if statements that
            // depend on these values. If you don't have any constants like that, then you can set the member to nullptr,
            // which our struct initialization does automatically.
            // .specialization_info()

            let frag_shader_stage = vk::PipelineShaderStageCreateInfo::default()
                .module(shader_module)
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .name(CStr::from_bytes_with_nul_unchecked(b"fs\0"));

            let shader_stages = [vert_shader_stage, frag_shader_stage];

            let input_bindings = [Vertex::get_binding_description()];
            let input_attributes = Vertex::get_attribute_descriptions();

            let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default()
                .vertex_binding_descriptions(&input_bindings)
                .vertex_attribute_descriptions(&input_attributes);

            let input_assembly_stage = vk::PipelineInputAssemblyStateCreateInfo::default()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                // used with Indexed drawing + Triangle Fan/Strip topologies. This is more efficient than explicitly
                // ending the current primitive and explicitly starting a new primitive of the same type.
                // A special “index” indicates that the primitive should start over.
                //   If VkIndexType is VK_INDEX_TYPE_UINT16, special index is 0xFFFF
                //   If VkIndexType is VK_INDEX_TYPE_UINT32, special index is 0xFFFFFFFF
                // One Really Good use of Restart Enable is in Drawing Terrain Surfaces with Triangle Strips.
                .primitive_restart_enable(false);

            let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
                .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

            let viewport_state = vk::PipelineViewportStateCreateInfo::default()
                .viewport_count(1)
                .scissor_count(1);

            let rasterization_state = vk::PipelineRasterizationStateCreateInfo::default()
                .cull_mode(vk::CullModeFlags::BACK)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .polygon_mode(vk::PolygonMode::FILL)
                .line_width(1.0)
                .rasterizer_discard_enable(false)
                .depth_clamp_enable(false)
                .depth_bias_enable(false)
                .depth_bias_clamp(0.0)
                .depth_bias_slope_factor(0.0)
                .depth_bias_constant_factor(0.0);

            let multisample = vk::PipelineMultisampleStateCreateInfo::default()
                .sample_shading_enable(true)
                .min_sample_shading(0.2)
                .rasterization_samples(self.gpu.device_context.msaa_samples)
                .sample_mask(&[])
                .alpha_to_coverage_enable(false)
                .alpha_to_one_enable(false);

            let color_attachments = [vk::PipelineColorBlendAttachmentState {
                blend_enable: false.into(),
                src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
                dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
                color_blend_op: vk::BlendOp::ADD,
                src_alpha_blend_factor: vk::BlendFactor::SRC_ALPHA,
                dst_alpha_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
                alpha_blend_op: vk::BlendOp::ADD,
                color_write_mask: vk::ColorComponentFlags::RGBA,
            }];
            let color_blend = vk::PipelineColorBlendStateCreateInfo::default()
                // corresponding to renderPass subPass pColorAttachments
                .attachments(&color_attachments)
                .blend_constants([0.0, 0.0, 0.0, 0.0])
                .logic_op_enable(false)
                .logic_op(vk::LogicOp::COPY);

            let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
                .depth_write_enable(def.depth_write)
                .depth_test_enable(def.depth_test)
                .depth_compare_op(if self.depth_reverse_z {
                    vk::CompareOp::GREATER
                } else {
                    vk::CompareOp::LESS
                })
                .stencil_test_enable(false)
                .front(vk::StencilOpState::default())
                .back(vk::StencilOpState::default())
                // only keep fragments that fall within the specified depth range
                .depth_bounds_test_enable(false)
                .min_depth_bounds(0.0)
                .max_depth_bounds(1.0);

            let push_constant_ranges = [vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::ALL_GRAPHICS)
                .offset(0)
                .size(size_of::<ObjectData>() as u32)];
            let descriptor_set_layouts = vec![self.descriptor_set_layout, descriptor_set_layout];
            let layout_create_info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&descriptor_set_layouts)
                .push_constant_ranges(&push_constant_ranges);

            let pipeline_layout = self
                .gpu
                .device_context
                .device
                .create_pipeline_layout(&layout_create_info, None)
                .expect("failed to create pipeline layout!");

            let create_info = vk::GraphicsPipelineCreateInfo::default()
                .stages(&shader_stages)
                .vertex_input_state(&vertex_input_state)
                .input_assembly_state(&input_assembly_stage)
                .dynamic_state(&dynamic_state)
                .viewport_state(&viewport_state)
                .rasterization_state(&rasterization_state)
                .multisample_state(&multisample)
                .color_blend_state(&color_blend)
                .depth_stencil_state(&depth_stencil)
                .layout(pipeline_layout)
                .render_pass(self.render_pass)
                .subpass(0)
                .base_pipeline_handle(vk::Pipeline::null())
                .base_pipeline_index(0);

            let pipeline = self
                .gpu
                .device_context
                .device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None)
                .expect("failed to create graphics pipeline!")[0];

            (pipeline, pipeline_layout)
        }
    }

    fn create_descriptor_pool(gpu: &GPU) -> vk::DescriptorPool {
        unsafe {
            // todo: VK_KHR_push_descriptor

            let mut pool_sizes: Vec<vk::DescriptorPoolSize> = vec![];

            pool_sizes.push(vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: ForwardRenderer::FRAMES_IN_FLIGHT * 100,
            });
            pool_sizes.push(vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLED_IMAGE,
                descriptor_count: ForwardRenderer::FRAMES_IN_FLIGHT * 100,
            });
            pool_sizes.push(vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLER,
                descriptor_count: ForwardRenderer::FRAMES_IN_FLIGHT * 100,
            });

            let create_info = vk::DescriptorPoolCreateInfo::default()
                .pool_sizes(&pool_sizes)
                .max_sets(ForwardRenderer::FRAMES_IN_FLIGHT * (100 + 1));

            gpu.device_context
                .device
                .create_descriptor_pool(&create_info, None)
                .expect("failed to create descriptor pool!")
        }
    }

    fn create_uniform_buffers(
        gpu: &GPU,
    ) -> (Vec<vk::Buffer>, Vec<vk::DeviceMemory>, Vec<*mut c_void>) {
        let buffer_size = size_of::<SceneData>() as vk::DeviceSize;
        let mut buffers = Vec::new();
        let mut memories = Vec::new();
        let mut memories_mapped = Vec::new();

        for _ in 0..Self::FRAMES_IN_FLIGHT {
            let (buffer, memory, memory_mapped) = gpu.create_mapped_buffers(buffer_size);

            buffers.push(buffer);
            memories.push(memory);
            memories_mapped.push(memory_mapped);
        }

        (buffers, memories, memories_mapped)
    }

    unsafe fn create_color_resources(gpu: &GPU) -> (vk::Image, vk::DeviceMemory, vk::ImageView) {
        let (color_image, color_image_memory) = gpu.device_context.create_image(
            gpu.swap_chain.extent.width,
            gpu.swap_chain.extent.height,
            1,
            gpu.device_context.msaa_samples,
            gpu.swap_chain.format,
            vk::ImageTiling::OPTIMAL,
            // Using VK_IMAGE_USAGE_TRANSIENT_ATTACHMENT_BIT combined with VK_MEMORY_PROPERTY_LAZILY_ALLOCATED_BIT memory.
            // The idea is that lazy memory allocation prevents allocations for the multisample color attachment, which is
            // only used as a temporary during the render pass, and therefore remains on-chip instead of stored in device memory.
            // https://registry.khronos.org/vulkan/specs/1.2-extensions/html/vkspec.html#memory-device-lazy_allocation
            // vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSIENT_ATTACHMENT,
            vk::ImageUsageFlags::COLOR_ATTACHMENT,
            // vk::MemoryPropertyFlags::DEVICE_LOCAL | vk::MemoryPropertyFlags::LAZILY_ALLOCATED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );
        let color_image_view = gpu.device_context.create_image_view(
            color_image,
            gpu.swap_chain.format,
            vk::ImageAspectFlags::COLOR,
            1,
        );

        (color_image, color_image_memory, color_image_view)
    }

    unsafe fn create_depth_resources(gpu: &GPU) -> (vk::Image, vk::DeviceMemory, vk::ImageView) {
        let depth_format = Self::find_depth_format(gpu);
        let (depth_image, depth_image_memory) = gpu.device_context.create_image(
            gpu.swap_chain.extent.width,
            gpu.swap_chain.extent.height,
            1,
            gpu.device_context.msaa_samples,
            depth_format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );
        let depth_image_view = gpu.device_context.create_image_view(
            depth_image,
            depth_format,
            vk::ImageAspectFlags::DEPTH,
            1,
        );

        (depth_image, depth_image_memory, depth_image_view)
    }

    unsafe fn create_render_pass(gpu: &GPU) -> vk::RenderPass {
        // Textures and framebuffers in Vulkan are represented by VkImage objects with a certain pixel format,
        //   however the layout of the pixels in memory can change based on what you're trying to do with an image.
        // Some of the most common layouts are:
        //   VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL: Images used as color attachment
        //   VK_IMAGE_LAYOUT_PRESENT_SRC_KHR: Images to be presented in the swap chain
        //   VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL: Images to be used as destination for a memory copy operation
        let color_attachment = vk::AttachmentDescription {
            format: gpu.swap_chain.format,
            samples: gpu.device_context.msaa_samples,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            flags: Default::default(),
        };
        let depth_attachment = vk::AttachmentDescription {
            format: Self::find_depth_format(gpu),
            samples: gpu.device_context.msaa_samples,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::DONT_CARE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            flags: Default::default(),
        };
        let resolve_color_attachment = vk::AttachmentDescription {
            format: gpu.swap_chain.format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::DONT_CARE,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            flags: Default::default(),
        };

        let attachments = [color_attachment, depth_attachment, resolve_color_attachment];

        let color_attachment_refs = [vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];
        let depth_attachment_ref = vk::AttachmentReference {
            attachment: 1,
            layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };
        let resolve_color_attachment_refs = [vk::AttachmentReference {
            attachment: 2,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];

        let sub_passes = [vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachment_refs)
            .depth_stencil_attachment(&depth_attachment_ref)
            .resolve_attachments(&resolve_color_attachment_refs)];
        // .input_attachments()
        // .preserve_attachments()

        let dependencies = [vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::LATE_FRAGMENT_TESTS
                | vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags::NONE,
            dst_subpass: 0,
            dst_stage_mask: vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            ..Default::default()
        }];

        let create_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(&sub_passes)
            .dependencies(&dependencies);

        gpu.device_context
            .device
            .create_render_pass(&create_info, None)
            .expect("failed to create render pass!")
    }

    unsafe fn create_framebuffers(
        gpu: &GPU,
        render_pass: vk::RenderPass,
        color_image_view: vk::ImageView,
        depth_image_view: vk::ImageView,
    ) -> Vec<vk::Framebuffer> {
        // be aware, here is not using MAX_INFLIGHT
        gpu.swap_chain
            .image_views
            .iter()
            .map(|&image_view| {
                let attachments = [color_image_view, depth_image_view, image_view];

                let create_info = vk::FramebufferCreateInfo::default()
                    .width(gpu.swap_chain.extent.width)
                    .height(gpu.swap_chain.extent.height)
                    .layers(1)
                    .attachments(&attachments)
                    .render_pass(render_pass);

                gpu.device_context
                    .device
                    .create_framebuffer(&create_info, None)
                    .expect("failed to create framebuffer!")
            })
            .collect::<Vec<vk::Framebuffer>>()
    }

    unsafe fn find_depth_format(gpu: &GPU) -> vk::Format {
        gpu.find_supported_format(
            vec![
                vk::Format::D32_SFLOAT,
                vk::Format::D32_SFLOAT_S8_UINT,
                vk::Format::D24_UNORM_S8_UINT,
            ],
            vk::ImageTiling::OPTIMAL,
            vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
        )
    }
}

impl Drop for ForwardRenderer {
    fn drop(&mut self) {
        unsafe {
            self.clear_cache();

            let device = &self.gpu.device_context.device;
            self.uniform_buffers.iter().for_each(|buffer| {
                device.destroy_buffer(*buffer, None);
            });
            self.uniform_buffer_memories.iter().for_each(|memory| {
                device.free_memory(*memory, None);
            });

            self.framebuffers
                .iter()
                .for_each(|&framebuffer| device.destroy_framebuffer(framebuffer, None));

            device.destroy_image(self.color_image, None);
            device.free_memory(self.color_image_memory, None);
            device.destroy_image_view(self.color_image_view, None);

            device.destroy_image(self.depth_image, None);
            device.free_memory(self.depth_image_memory, None);
            device.destroy_image_view(self.depth_image_view, None);
            device.destroy_render_pass(self.render_pass, None);

            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}

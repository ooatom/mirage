use super::*;
use crate::Shaders;
use std::f32::consts::PI;

use crate::math::{Mat4, Vec3};
use crate::renderer::utils::{create_buffer, create_image, create_image_view};
use ash::vk;
use image;
use std::ffi::{c_void, CStr};
use std::io::Cursor;
use std::mem::{align_of, size_of};
use std::rc::Rc;

pub struct SimplePass {
    device: Rc<VkDeviceContext>,
    renderer: Rc<ForwardRenderer>,
    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,

    uniform_buffers: Vec<vk::Buffer>,
    uniform_buffer_memories: Vec<vk::DeviceMemory>,
    uniform_buffer_memories_mapped: Vec<*mut c_void>,

    descriptor_sets: Vec<vk::DescriptorSet>,

    objects: Vec<SimplePassObject>,
}

impl SimplePass {
    pub fn new(device: Rc<VkDeviceContext>, renderer: Rc<ForwardRenderer>) -> Self {
        unsafe {
            let descriptor_set_layout = SimplePass::create_descriptor_set_layout(&device);
            let (pipeline, pipeline_layout) =
                SimplePass::create_pipeline(&device, &renderer, descriptor_set_layout);
            let (uniform_buffers, uniform_buffer_memories, uniform_buffer_memories_mapped) =
                SimplePass::create_uniform_buffers(&device);
            let descriptor_sets = SimplePass::create_descriptor_sets(
                &device,
                &renderer,
                descriptor_set_layout,
                &uniform_buffers,
            );

            Self {
                device,
                renderer,

                descriptor_set_layout,
                pipeline,
                pipeline_layout,

                uniform_buffers,
                uniform_buffer_memories,
                uniform_buffer_memories_mapped,

                descriptor_sets,
                objects: Vec::new(),
            }
        }
    }

    pub fn add_object(&mut self, object: SimplePassObject) {
        self.objects.push(object);
    }

    pub fn update(&self, frame_index: usize, _: f32) {
        self.objects.iter().for_each(|obj| {
            let image_infos = [vk::DescriptorImageInfo {
                image_view: obj.texture_image_view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                sampler: obj.texture_image_sampler,
            }];

            let texture_write = vk::WriteDescriptorSet::default()
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .image_info(&image_infos)
                .dst_set(self.descriptor_sets[frame_index])
                .dst_binding(1)
                .dst_array_element(0);

            let sampler_write = vk::WriteDescriptorSet::default()
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .image_info(&image_infos)
                .dst_set(self.descriptor_sets[frame_index])
                .dst_binding(2)
                .dst_array_element(0);

            unsafe {
                self.device
                    .device
                    .update_descriptor_sets(&[texture_write, sampler_write], &[]);
            }
        });

        // let aspect = self.swapchain_properties.extent.width as f32
        //     / self.swapchain_properties.extent.height as f32;
        let model =
            Mat4::translate(Vec3::new(0.0, 0.0, -0.9)) * Mat4::scale(Vec3::new(5.0, 5.0, 5.0));
        let view = Mat4::look_at_rh(
            Vec3::new(0.0, 10.0, 10.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        // let projection = Mat4::orthographic_rh(-2.0, 2.0, -2.0, 2.0, 0.01, 100.0);
        let projection = Mat4::perspective_reversed_z_infinite_rh(PI / 2.0, 1.0, 0.01);
        let ubo = UniformBufferObject {
            model,
            view,
            projection,
        };
        unsafe {
            let mut align = ash::util::Align::new(
                self.uniform_buffer_memories_mapped[frame_index],
                align_of::<UniformBufferObject>() as vk::DeviceSize,
                size_of::<UniformBufferObject>() as vk::DeviceSize,
            );
            align.copy_from_slice(&[ubo]);
        }
    }

    pub fn render(&self, command_buffer: vk::CommandBuffer, frame_index: usize) {
        unsafe {
            let device = &self.device.device;

            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
            self.objects.iter().for_each(|obj| {
                device.cmd_bind_vertex_buffers(command_buffer, 0, &[obj.vertex_buffer], &[0]);
                device.cmd_bind_index_buffer(
                    command_buffer,
                    obj.index_buffer,
                    0,
                    vk::IndexType::UINT32,
                );
                device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline_layout,
                    0,
                    &[self.descriptor_sets[frame_index]],
                    &[],
                );
                // device.cmd_push_constants();
                device.cmd_draw_indexed(command_buffer, obj.indices.len() as u32, 1, 0, 0, 0);
            });
        }
    }

    pub unsafe fn create_texture_image(
        &self,
        path: &str,
    ) -> (vk::Image, vk::DeviceMemory, vk::ImageView, vk::Sampler) {
        let image = image::open(path).expect("failed to load image!");
        let image_rgba8 = image.to_rgba8();
        let width = image_rgba8.width();
        let height = image_rgba8.height();
        let mip_levels = ((width.min(height) as f32).log2().floor() + 1.0) as u32;
        let pixels = image_rgba8.into_raw();
        let image_size = (pixels.len() * size_of::<u8>()) as vk::DeviceSize;

        let (staging_buffer, staging_memory, _) = create_buffer(
            &self.device,
            image_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        );
        let staging_memory_mapped = self
            .device
            .device
            .map_memory(staging_memory, 0, image_size, vk::MemoryMapFlags::empty())
            .expect("failed to map staging memory!");

        let mut align = ash::util::Align::new(
            staging_memory_mapped,
            align_of::<u8>() as vk::DeviceSize,
            image_size,
        );
        align.copy_from_slice(&pixels);
        self.device.device.unmap_memory(staging_memory);

        let (image, memory) = create_image(
            &self.device,
            width,
            height,
            mip_levels,
            vk::SampleCountFlags::TYPE_1,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        {
            self.renderer.transition_image_layout(
                image,
                vk::Format::R8G8B8A8_SRGB,
                mip_levels,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            );
            self.renderer
                .copy_buffer_to_image(staging_buffer, image, width, height);
            if mip_levels > 1 {
                self.renderer.generate_mipmaps(
                    image,
                    vk::Format::R8G8B8A8_SRGB,
                    width,
                    height,
                    mip_levels,
                );
            } else {
                self.renderer.transition_image_layout(
                    image,
                    vk::Format::R8G8B8A8_SRGB,
                    1,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                );
            }

            self.device.device.free_memory(staging_memory, None);
            self.device.device.destroy_buffer(staging_buffer, None);
        }

        let image_view = create_image_view(
            &self.device.device,
            image,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageAspectFlags::COLOR,
            mip_levels,
        );

        let create_info = vk::SamplerCreateInfo::default()
            .anisotropy_enable(true)
            .max_anisotropy(
                self.device
                    .physical_device_properties
                    .limits
                    .max_sampler_anisotropy,
            )
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .min_filter(vk::Filter::LINEAR)
            .mag_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .min_lod(0.0)
            .max_lod(mip_levels as f32)
            .mip_lod_bias(0.0)
            .unnormalized_coordinates(false)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .border_color(vk::BorderColor::FLOAT_OPAQUE_BLACK);

        let sampler = self
            .device
            .device
            .create_sampler(&create_info, None)
            .expect("failed to create image sampler!");

        (image, memory, image_view, sampler)
    }

    pub unsafe fn create_buffer_with_data<T: Copy>(
        &self,
        array: &Vec<T>,
        usage: vk::BufferUsageFlags,
    ) -> (vk::Buffer, vk::DeviceMemory) {
        let buffer_size = (size_of::<T>() * array.len()) as vk::DeviceSize;
        let (staging_buffer, staging_memory, _) = create_buffer(
            &self.device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        );

        let staging_memory_mapped = self
            .device
            .device
            .map_memory(staging_memory, 0, buffer_size, vk::MemoryMapFlags::empty())
            .expect("failed to map buffer staging memory!");
        let mut align = ash::util::Align::new(
            staging_memory_mapped,
            align_of::<T>() as vk::DeviceSize,
            buffer_size,
        );
        align.copy_from_slice(array);
        self.device.device.unmap_memory(staging_memory);

        let (buffer, buffer_memory, _) = create_buffer(
            &self.device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | usage,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        // The transfer of data to the GPU is an operation that happens in the background and the specification
        // simply tells us that it is guaranteed to be complete as of the next call to vkQueueSubmit.
        // https://registry.khronos.org/vulkan/specs/1.3-extensions/html/chap7.html#synchronization-submission-host-writes
        self.renderer
            .copy_buffer(staging_buffer, buffer, buffer_size);
        self.device.device.destroy_buffer(staging_buffer, None);
        self.device.device.free_memory(staging_memory, None);

        (buffer, buffer_memory)
    }

    unsafe fn create_descriptor_set_layout(device: &VkDeviceContext) -> vk::DescriptorSetLayout {
        let uniform_layout_binding = vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::VERTEX,
            ..Default::default()
        };

        let texture_layout_binding = vk::DescriptorSetLayoutBinding {
            binding: 1,
            descriptor_type: vk::DescriptorType::SAMPLED_IMAGE,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            ..Default::default()
        };

        let sampler_layout_binding = vk::DescriptorSetLayoutBinding {
            binding: 2,
            descriptor_type: vk::DescriptorType::SAMPLER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            ..Default::default()
        };

        let bindings = [
            uniform_layout_binding,
            texture_layout_binding,
            sampler_layout_binding,
        ];
        let create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

        device
            .device
            .create_descriptor_set_layout(&create_info, None)
            .expect("failed to create descriptor set layout!")
    }

    unsafe fn create_pipeline(
        device: &VkDeviceContext,
        renderer: &ForwardRenderer,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> (vk::Pipeline, vk::PipelineLayout) {
        // The Vulkan SDK includes libshaderc, which is a library to compile GLSL code to SPIR-V from within your program.
        // https://github.com/google/shaderc
        // little endian
        // let mut buffer = Cursor::new(Shaders::get("simple.vert.spv").unwrap().data);
        // let vert_shader_code = ash::util::read_spv(&mut buffer).unwrap();
        // let mut buffer = Cursor::new(Shaders::get("simple.frag.spv").unwrap().data);
        // let frag_shader_code = ash::util::read_spv(&mut buffer).unwrap();

        // let vert_shader_module = device.create_shader_module(&vert_shader_code);
        // let frag_shader_module = device.create_shader_module(&frag_shader_code);

        let mut buffer = Cursor::new(Shaders::get("simple.spv").unwrap().data);
        let shader_code = ash::util::read_spv(&mut buffer).unwrap();
        let shader_module = device.create_shader_module(&shader_code);

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
            .cull_mode(vk::CullModeFlags::NONE)
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
            .rasterization_samples(device.msaa_samples)
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
            .depth_write_enable(true)
            .depth_test_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .stencil_test_enable(false)
            .front(vk::StencilOpState::default())
            .back(vk::StencilOpState::default())
            // only keep fragments that fall within the specified depth range
            .depth_bounds_test_enable(false)
            .min_depth_bounds(0.0)
            .max_depth_bounds(1.0);

        let descriptor_set_layouts = [descriptor_set_layout];
        let layout_create_info =
            vk::PipelineLayoutCreateInfo::default().set_layouts(&descriptor_set_layouts);
        // .push_constant_ranges()
        let layout = device
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
            .layout(layout)
            .render_pass(renderer.render_pass)
            .subpass(0)
            .base_pipeline_handle(vk::Pipeline::null())
            .base_pipeline_index(0);

        let pipeline = device
            .device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None)
            .expect("failed to create graphics pipeline!")[0];

        device.device.destroy_shader_module(shader_module, None);

        (pipeline, layout)
    }

    unsafe fn create_uniform_buffers(
        device: &VkDeviceContext,
    ) -> (Vec<vk::Buffer>, Vec<vk::DeviceMemory>, Vec<*mut c_void>) {
        let buffer_size = size_of::<UniformBufferObject>() as vk::DeviceSize;
        let mut buffers = Vec::new();
        let mut memories = Vec::new();
        let mut memories_mapped = Vec::new();

        for _ in 0..ForwardRenderer::MAX_FRAMES_IN_FLIGHT {
            let (buffer, memory, _) = create_buffer(
                device,
                buffer_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );

            let memory_mapped = device
                .device
                .map_memory(memory, 0, buffer_size, vk::MemoryMapFlags::empty())
                .expect("failed to map buffer memory!");

            buffers.push(buffer);
            memories.push(memory);
            memories_mapped.push(memory_mapped);
        }

        (buffers, memories, memories_mapped)
    }

    unsafe fn create_descriptor_sets(
        device: &VkDeviceContext,
        renderer: &ForwardRenderer,
        layout: vk::DescriptorSetLayout,
        uniform_buffers: &Vec<vk::Buffer>,
    ) -> Vec<vk::DescriptorSet> {
        let layouts = [layout; ForwardRenderer::MAX_FRAMES_IN_FLIGHT as usize];
        let allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(renderer.descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets = device
            .device
            .allocate_descriptor_sets(&allocate_info)
            .expect("failed to allocate descriptor sets!");

        for (index, descriptor_set) in descriptor_sets.iter().enumerate() {
            let buffer_infos = [vk::DescriptorBufferInfo {
                buffer: uniform_buffers[index],
                offset: 0,
                range: size_of::<UniformBufferObject>() as vk::DeviceSize,
            }];
            let ubo_write = vk::WriteDescriptorSet::default()
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&buffer_infos)
                .dst_set(*descriptor_set)
                .dst_binding(0)
                // starting element in that array
                .dst_array_element(0);

            device.device.update_descriptor_sets(&[ubo_write], &[]);
        }

        descriptor_sets
    }
}

impl Drop for SimplePass {
    fn drop(&mut self) {
        unsafe {
            let device = &self.device.device;
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);

            self.uniform_buffers.iter().for_each(|buffer| {
                device.destroy_buffer(*buffer, None);
            });
            self.uniform_buffer_memories.iter().for_each(|memory| {
                device.free_memory(*memory, None);
            });
            // device
            //     .free_descriptor_sets(self.renderer.descriptor_pool, &self.descriptor_sets)
            //     .unwrap();

            self.objects.iter().for_each(|obj| {
                device.destroy_image(obj.texture_image, None);
                device.destroy_sampler(obj.texture_image_sampler, None);
                device.destroy_image_view(obj.texture_image_view, None);
                device.free_memory(obj.texture_image_memory, None);

                device.destroy_buffer(obj.vertex_buffer, None);
                device.free_memory(obj.vertex_buffer_memory, None);
                device.destroy_buffer(obj.index_buffer, None);
                device.free_memory(obj.index_buffer_memory, None);
            });
        }
    }
}

// Default repr Rust might rearrange the order of fields
#[repr(C)]
#[derive(Copy, Clone, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex {
    fn get_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }
    }

    fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
        [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: size_of::<[f32; 3]>() as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: size_of::<[f32; 3]>() as u32 * 2,
            },
        ]
    }
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq)]
struct UniformBufferObject {
    pub model: Mat4,
    pub view: Mat4,
    pub projection: Mat4,
}

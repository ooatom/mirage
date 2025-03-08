use super::*;
use crate::gpu::GPU;
use crate::math::Mat4;
use ash::vk;
use std::ffi::c_void;
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

// struct FrameData {}

pub struct ForwardRenderer {
    gpu: Rc<GPU>,

    pub view: Mat4,
    pub projection: Mat4,

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

            let descriptor_sets = gpu.create_descriptor_sets(&vec![
                descriptor_set_layout;
                Self::FRAMES_IN_FLIGHT as usize
            ]);
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

                view: Mat4::identity(),
                projection: Mat4::identity(),

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

            let mut gpu_assets = context.gpu_assets.borrow_mut();
            context.objects.iter().for_each(|object| {
                let Some((pipeline, properties)) = gpu_assets.get_material(&object.material, self)
                else {
                    return;
                };
                let Some(Some(texture)) = properties.get("texture") else {
                    return;
                };

                let image_infos = [vk::DescriptorImageInfo {
                    image_view: texture.image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    sampler: texture.image_sampler,
                }];

                let texture_write = vk::WriteDescriptorSet::default()
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&image_infos)
                    .dst_set(pipeline.get_descriptor_set(frame_index))
                    .dst_binding(0)
                    .dst_array_element(0);

                let sampler_write = vk::WriteDescriptorSet::default()
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .image_info(&image_infos)
                    .dst_set(pipeline.get_descriptor_set(frame_index))
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

            let mut gpu_assets = context.gpu_assets.borrow_mut();
            context.objects.iter().for_each(|object| {
                let Some(pipeline) = gpu_assets.get_pipeline(&object.material, self) else {
                    return;
                };
                let Some(geom) = gpu_assets.get_geom(&object.geom) else {
                    return;
                };

                let object_data = ObjectData {
                    model: object.model,
                };
                device.cmd_push_constants(
                    command_buffer,
                    pipeline.pipeline_layout,
                    vk::ShaderStageFlags::ALL_GRAPHICS,
                    0,
                    any_as_u8_slice(&object_data),
                );

                device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.pipeline_layout,
                    0,
                    &[
                        self.descriptor_sets[frame_index],
                        pipeline.get_descriptor_set(frame_index),
                    ],
                    &[],
                );

                device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.pipeline,
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
        }
    }
}

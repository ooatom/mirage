use crate::gpu;
use ash::vk;
use ash::vk::BufferCopy;
use std::cell::Cell;
use std::rc::Rc;

pub struct ForwardRenderer {
    device: Rc<gpu::Device>,
    swap_chain: Rc<gpu::SwapChain>,

    command_pool: vk::CommandPool,
    transient_command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    pub descriptor_pool: vk::DescriptorPool,

    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,

    pub render_pass: vk::RenderPass,
    color_image: vk::Image,
    color_image_memory: vk::DeviceMemory,
    color_image_view: vk::ImageView,
    depth_image: vk::Image,
    depth_image_memory: vk::DeviceMemory,
    depth_image_view: vk::ImageView,

    framebuffers: Vec<vk::Framebuffer>,

    frame_index: Cell<usize>,
}

impl ForwardRenderer {
    pub(crate) const MAX_FRAMES_IN_FLIGHT: u32 = 2;

    pub fn new(
        instance: &ash::Instance,
        device: Rc<gpu::Device>,
        swap_chain: Rc<gpu::SwapChain>,
    ) -> Self {
        unsafe {
            let (command_pool, transient_command_pool) = Self::create_command_pools(&device);
            let command_buffers = Self::create_command_buffers(&device, command_pool);
            let (image_available_semaphores, render_finished_semaphores, in_flight_fences) =
                Self::create_sync_objects(&device);
            let descriptor_pool = Self::create_descriptor_pool(&device);

            let render_pass = Self::create_render_pass(instance, &device, &swap_chain);
            let (color_image, color_image_memory, color_image_view) =
                Self::create_color_resources(&device, &swap_chain);
            let (depth_image, depth_image_memory, depth_image_view) =
                Self::create_depth_resources(instance, &device, &swap_chain);
            let framebuffers = Self::create_framebuffers(
                &device,
                &swap_chain,
                render_pass,
                color_image_view,
                depth_image_view,
            );

            Self {
                device,
                swap_chain,

                command_pool,
                transient_command_pool,
                command_buffers,
                descriptor_pool,

                image_available_semaphores,
                render_finished_semaphores,
                in_flight_fences,

                render_pass,
                color_image,
                color_image_memory,
                color_image_view,
                depth_image,
                depth_image_memory,
                depth_image_view,

                framebuffers,

                frame_index: Cell::new(0),
            }
        }
    }

    pub fn render(&self, simple_pass: &gpu::SimplePass) {
        unsafe {
            let frame_index = self.frame_index.get();

            let device = &self.device.device;
            let fence = self.in_flight_fences[frame_index];
            let image_available_semaphore = self.image_available_semaphores[frame_index];
            let render_finished_semaphore = self.render_finished_semaphores[frame_index];

            // There happens to be two kinds of semaphores in Vulkan, binary and timeline. We use binary semaphores here.
            // A fence has a similar purpose, in that it is used to synchronize execution, but it is for ordering the execution on the CPU, otherwise known as the host.
            device
                .wait_for_fences(&[fence], true, u64::MAX)
                .expect("failed to wait fence!");

            let acquire_result = self.swap_chain.swap_chain_fn.acquire_next_image(
                self.swap_chain.swap_chain,
                u64::MAX,
                image_available_semaphore,
                vk::Fence::null(),
            );

            let (image_index, _) = match acquire_result {
                Ok(result) => result,
                Err(err_code) => {
                    if err_code == vk::Result::ERROR_OUT_OF_DATE_KHR {
                        // self.recreate_swap_chain();
                        return;
                    }
                    panic!("failed to acquire swap chain image!");
                }
            };

            device
                .reset_fences(&[fence])
                .expect("failed to reset fence!");

            let command_buffer = self.command_buffers[frame_index];
            device
                .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())
                .expect("failed to reset command buffer!");

            let begin_info = vk::CommandBufferBeginInfo::default()
                // ONE_TIME_SUBMIT_BIT: The command buffer will be rerecorded right after executing it once.
                // RENDER_PASS_CONTINUE_BIT: This is a secondary command buffer that will be entirely within a single render pass.
                // SIMULTANEOUS_USE_BIT: The command buffer can be resubmitted while it is also already pending execution.
                .flags(vk::CommandBufferUsageFlags::SIMULTANEOUS_USE);
            // Only relevant for secondary command buffers. It specifies which state to inherit from the calling primary command buffers.
            // .inheritance_info()

            device
                .begin_command_buffer(command_buffer, &begin_info)
                .expect("failed to begin command buffer!");

            device.cmd_set_viewport(
                command_buffer,
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: self.swap_chain.extent.width as f32,
                    height: self.swap_chain.extent.height as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );
            device.cmd_set_scissor(
                command_buffer,
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swap_chain.extent,
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
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];

            let render_pass_begin_info = vk::RenderPassBeginInfo::default()
                .clear_values(&clear_values)
                .render_pass(self.render_pass)
                .framebuffer(self.framebuffers[image_index as usize])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swap_chain.extent,
                });

            // INLINE: The render pass commands will be embedded in the primary command buffer itself
            // and no secondary command buffers will be executed.
            // SECONDARY_COMMAND_BUFFERS: The render pass commands will be executed from secondary command buffers.
            device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );

            simple_pass.update(frame_index, 0.0);
            simple_pass.render(command_buffer, frame_index);

            device.cmd_end_render_pass(command_buffer);
            device
                .end_command_buffer(command_buffer)
                .expect("failed to end command buffer!");

            let wait_semaphores = [image_available_semaphore];
            let signal_semaphores = [render_finished_semaphore];
            let command_buffers = [command_buffer];
            let stage_masks = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

            let submit_info = vk::SubmitInfo::default()
                .command_buffers(&command_buffers)
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&stage_masks)
                .signal_semaphores(&signal_semaphores);
            device
                .queue_submit(self.device.graphic_queue.unwrap(), &[submit_info], fence)
                .unwrap();

            let image_indices = [image_index];
            let swap_chains = [self.swap_chain.swap_chain];
            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&signal_semaphores)
                .image_indices(&image_indices)
                .swapchains(&swap_chains);

            // Queueing an image for presentation defines a set of queue operations, including waiting on the semaphores and submitting a presentation
            // request to the presentation engine. However, the scope of this set of queue operations does not include the actual processing of the
            // image by the presentation engine.
            // vkQueuePresentKHR releases the acquisition of the image, which signals imageAvailableSemaphores for that image in later frames.
            let present_result = self
                .swap_chain
                .swap_chain_fn
                .queue_present(self.device.present_queue.unwrap(), &present_info);

            let is_suboptimal = present_result.unwrap_or_else(|err_code| {
                if err_code == vk::Result::ERROR_OUT_OF_DATE_KHR {
                    true
                } else {
                    panic!("failed to submit present queue!");
                }
            });
            if is_suboptimal {
                // framebufferResized = false;
                // self.recreate_swap_chain();
            }

            self.frame_index
                .set((frame_index + 1) % (Self::MAX_FRAMES_IN_FLIGHT as usize));
        }
    }

    pub unsafe fn transition_image_layout(
        &self,
        image: vk::Image,
        format: vk::Format,
        mip_levels: u32,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) {
        use vk::ImageLayout;

        let command_buffer = self.begin_single_time_command();

        let src_stage_mask;
        let src_access_mask;
        let dst_stage_mask;
        let dst_access_mask;

        if old_layout == ImageLayout::UNDEFINED && new_layout == ImageLayout::TRANSFER_DST_OPTIMAL {
            src_stage_mask = vk::PipelineStageFlags::TOP_OF_PIPE;
            src_access_mask = vk::AccessFlags::NONE;
            dst_stage_mask = vk::PipelineStageFlags::TRANSFER;
            dst_access_mask = vk::AccessFlags::TRANSFER_WRITE;
        } else if old_layout == ImageLayout::TRANSFER_DST_OPTIMAL
            && new_layout == ImageLayout::TRANSFER_SRC_OPTIMAL
        {
            src_stage_mask = vk::PipelineStageFlags::TRANSFER;
            src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            dst_stage_mask = vk::PipelineStageFlags::TRANSFER;
            dst_access_mask = vk::AccessFlags::TRANSFER_READ;
        } else if old_layout == ImageLayout::TRANSFER_DST_OPTIMAL
            && new_layout == ImageLayout::SHADER_READ_ONLY_OPTIMAL
        {
            src_stage_mask = vk::PipelineStageFlags::TRANSFER;
            src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            dst_stage_mask =
                vk::PipelineStageFlags::VERTEX_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER;
            dst_access_mask = vk::AccessFlags::SHADER_READ;
        } else if old_layout == ImageLayout::UNDEFINED
            && new_layout == ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
        {
            src_stage_mask = vk::PipelineStageFlags::TOP_OF_PIPE;
            src_access_mask = vk::AccessFlags::NONE;
            dst_stage_mask = vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS;
            dst_access_mask = vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE;
        } else {
            panic!("unsupported layout transition!");
        }

        let mut aspect_mask;
        if new_layout == ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL {
            aspect_mask = vk::ImageAspectFlags::DEPTH;
            if Self::has_stencil_component(format) {
                aspect_mask |= vk::ImageAspectFlags::STENCIL;
            }
        } else {
            aspect_mask = vk::ImageAspectFlags::COLOR;
        }

        let image_memory_barrier = vk::ImageMemoryBarrier::default()
            .image(image)
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_access_mask(src_access_mask)
            .dst_access_mask(dst_access_mask)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask,
                base_mip_level: 0,
                level_count: mip_levels,
                base_array_layer: 0,
                layer_count: 1,
            });

        // https://themaister.net/blog/2019/08/14/yet-another-blog-explaining-vulkan-synchronization/
        // 1. Wait for srcStageMask to complete
        // 2. Make all writes performed in possible combinations of srcStageMask + srcAccessMask available
        // 3. Make available memory visible to possible combinations of dstStageMask + dstAccessMask.
        // 4. Unblock work in dstStageMask.
        self.device.device.cmd_pipeline_barrier(
            command_buffer,
            src_stage_mask,
            dst_stage_mask,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[image_memory_barrier],
        );
        self.end_single_time_command(command_buffer);
    }

    pub unsafe fn copy_buffer(
        &self,
        src_buffer: vk::Buffer,
        dst_buffer: vk::Buffer,
        size: vk::DeviceSize,
    ) {
        let command_buffer = self.begin_single_time_command();
        let region = BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size,
        };
        self.device
            .device
            .cmd_copy_buffer(command_buffer, src_buffer, dst_buffer, &[region]);

        self.end_single_time_command(command_buffer);
    }

    pub unsafe fn copy_buffer_to_image(
        &self,
        buffer: vk::Buffer,
        image: vk::Image,
        width: u32,
        height: u32,
    ) {
        let command_buffer = self.begin_single_time_command();

        let region = vk::BufferImageCopy {
            buffer_offset: 0,
            // If either of these values is zero, that aspect of the buffer memory is considered to
            // be tightly packed according to the imageExtent.
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            image_extent: vk::Extent3D {
                width,
                height,
                depth: 1,
            },
        };

        self.device.device.cmd_copy_buffer_to_image(
            command_buffer,
            buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[region],
        );
        self.end_single_time_command(command_buffer);
    }

    pub unsafe fn generate_mipmaps(
        &self,
        image: vk::Image,
        format: vk::Format,
        width: u32,
        height: u32,
        mip_levels: u32,
    ) {
        let format_properties = self.device.get_format_properties(format);
        if !format_properties
            .optimal_tiling_features
            .contains(vk::FormatFeatureFlags::SAMPLED_IMAGE_FILTER_LINEAR)
        {
            panic!("failed to generate mipmaps, texture image does not support linear filter!")
        }

        let command_buffer = self.begin_single_time_command();

        let mut barrier = vk::ImageMemoryBarrier::default()
            .image(image)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let mut mip_width = width as i32;
        let mut mip_height = height as i32;

        for i in 1..mip_levels {
            barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
            barrier.new_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            barrier.dst_access_mask = vk::AccessFlags::TRANSFER_READ;
            barrier.subresource_range.base_mip_level = i - 1;
            self.device.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );

            let next_mip_width = if mip_width > 1 {
                mip_width / 2
            } else {
                mip_width
            };
            let next_mip_height = if mip_height > 1 {
                mip_height / 2
            } else {
                mip_height
            };

            let region = vk::ImageBlit {
                src_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: i - 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_offsets: [
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: mip_width,
                        y: mip_height,
                        z: 1,
                    },
                ],
                dst_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: i,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                dst_offsets: [
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: next_mip_width,
                        y: next_mip_height,
                        z: 1,
                    },
                ],
            };

            self.device.device.cmd_blit_image(
                command_buffer,
                image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
                vk::Filter::LINEAR,
            );

            barrier.old_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
            barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_READ;
            barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;
            barrier.subresource_range.base_mip_level = i - 1;
            self.device.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::VERTEX_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );

            mip_width = next_mip_width;
            mip_height = next_mip_height;
        }

        barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
        barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
        barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
        barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;
        barrier.subresource_range.base_mip_level = mip_levels - 1;
        self.device.device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::VERTEX_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );

        self.end_single_time_command(command_buffer);
    }

    unsafe fn create_command_pools(device: &gpu::Device) -> (vk::CommandPool, vk::CommandPool) {
        // VK_COMMAND_POOL_CREATE_TRANSIENT_BIT:
        //   Hint that command buffers are rerecorded with new commands very often (may change memory allocation behavior)
        // VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT:
        //   Allow command buffers to be rerecorded individually, without this flag they all have to be reset together
        let create_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(device.graphic_queue_family.unwrap());
        let command_pool = device
            .device
            .create_command_pool(&create_info, None)
            .expect("failed to create command pool!");

        let create_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::TRANSIENT)
            .queue_family_index(device.graphic_queue_family.unwrap());
        let transient_command_pool = device
            .device
            .create_command_pool(&create_info, None)
            .expect("failed to create transient command pool!");

        (command_pool, transient_command_pool)
    }

    unsafe fn create_command_buffers(
        device: &gpu::Device,
        command_pool: vk::CommandPool,
    ) -> Vec<vk::CommandBuffer> {
        // VK_COMMAND_BUFFER_LEVEL_PRIMARY: Can be submitted to a queue for execution, but cannot be called from other command buffers.
        // VK_COMMAND_BUFFER_LEVEL_SECONDARY: Cannot be submitted directly, but can be called from primary command buffers.
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .command_buffer_count(Self::MAX_FRAMES_IN_FLIGHT)
            .level(vk::CommandBufferLevel::PRIMARY);

        device
            .device
            .allocate_command_buffers(&allocate_info)
            .expect("failed to allocate command buffers!")
    }

    unsafe fn create_sync_objects(
        device: &gpu::Device,
    ) -> (Vec<vk::Semaphore>, Vec<vk::Semaphore>, Vec<vk::Fence>) {
        let semaphore_create_info = vk::SemaphoreCreateInfo::default();

        let image_available_semaphores = (0..Self::MAX_FRAMES_IN_FLIGHT)
            .map(|_| {
                device
                    .device
                    .create_semaphore(&semaphore_create_info, None)
                    .expect("failed to create image available semaphore!")
            })
            .collect::<Vec<vk::Semaphore>>();

        let render_finished_semaphores = (0..Self::MAX_FRAMES_IN_FLIGHT)
            .map(|_| {
                device
                    .device
                    .create_semaphore(&semaphore_create_info, None)
                    .expect("failed to create render finished semaphore!")
            })
            .collect::<Vec<vk::Semaphore>>();

        let fence_create_info =
            vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
        let in_flight_fences: Vec<vk::Fence> = (0..Self::MAX_FRAMES_IN_FLIGHT)
            .map(|_| {
                device
                    .device
                    .create_fence(&fence_create_info, None)
                    .expect("failed to create in-flight fence!")
            })
            .collect::<Vec<vk::Fence>>();

        (
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
        )
    }

    unsafe fn create_descriptor_pool(device: &gpu::Device) -> vk::DescriptorPool {
        // todo: VK_KHR_push_descriptor

        let mut pool_sizes: Vec<vk::DescriptorPoolSize> = vec![];

        pool_sizes.push(vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: Self::MAX_FRAMES_IN_FLIGHT,
        });
        pool_sizes.push(vk::DescriptorPoolSize {
            ty: vk::DescriptorType::SAMPLED_IMAGE,
            descriptor_count: Self::MAX_FRAMES_IN_FLIGHT,
        });
        pool_sizes.push(vk::DescriptorPoolSize {
            ty: vk::DescriptorType::SAMPLER,
            descriptor_count: Self::MAX_FRAMES_IN_FLIGHT,
        });

        let create_info = vk::DescriptorPoolCreateInfo::default()
            // .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
            .pool_sizes(&pool_sizes)
            .max_sets(Self::MAX_FRAMES_IN_FLIGHT);

        device
            .device
            .create_descriptor_pool(&create_info, None)
            .expect("failed to create descriptor pool!")
    }

    unsafe fn create_render_pass(
        instance: &ash::Instance,
        device: &gpu::Device,
        swap_chain: &gpu::SwapChain,
    ) -> vk::RenderPass {
        // Textures and framebuffers in Vulkan are represented by VkImage objects with a certain pixel format,
        //   however the layout of the pixels in memory can change based on what you're trying to do with an image.
        // Some of the most common layouts are:
        //   VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL: Images used as color attachment
        //   VK_IMAGE_LAYOUT_PRESENT_SRC_KHR: Images to be presented in the swap chain
        //   VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL: Images to be used as destination for a memory copy operation
        let color_attachment = vk::AttachmentDescription {
            format: swap_chain.format,
            samples: device.msaa_samples,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            flags: Default::default(),
        };
        let depth_attachment = vk::AttachmentDescription {
            format: Self::find_depth_format(instance, device),
            samples: device.msaa_samples,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::DONT_CARE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            flags: Default::default(),
        };
        let resolve_color_attachment = vk::AttachmentDescription {
            format: swap_chain.format,
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

        device
            .device
            .create_render_pass(&create_info, None)
            .expect("failed to create render pass!")
    }

    unsafe fn create_color_resources(
        device: &gpu::Device,
        swap_chain: &gpu::SwapChain,
    ) -> (vk::Image, vk::DeviceMemory, vk::ImageView) {
        let (color_image, color_image_memory) = device.create_image(
            swap_chain.extent.width,
            swap_chain.extent.height,
            1,
            device.msaa_samples,
            swap_chain.format,
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
        let color_image_view = device.create_image_view(
            color_image,
            swap_chain.format,
            vk::ImageAspectFlags::COLOR,
            1,
        );

        (color_image, color_image_memory, color_image_view)
    }

    unsafe fn create_depth_resources(
        instance: &ash::Instance,
        device: &gpu::Device,
        swap_chain: &gpu::SwapChain,
    ) -> (vk::Image, vk::DeviceMemory, vk::ImageView) {
        let depth_format = Self::find_depth_format(instance, device);
        let (depth_image, depth_image_memory) = device.create_image(
            swap_chain.extent.width,
            swap_chain.extent.height,
            1,
            device.msaa_samples,
            depth_format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );
        let depth_image_view =
            device.create_image_view(depth_image, depth_format, vk::ImageAspectFlags::DEPTH, 1);

        (depth_image, depth_image_memory, depth_image_view)
    }

    unsafe fn create_framebuffers(
        device: &gpu::Device,
        swap_chain: &gpu::SwapChain,
        render_pass: vk::RenderPass,
        color_image_view: vk::ImageView,
        depth_image_view: vk::ImageView,
    ) -> Vec<vk::Framebuffer> {
        // be aware, here is not using MAX_INFLIGHT
        swap_chain
            .image_views
            .iter()
            .map(|&image_view| {
                let attachments = [color_image_view, depth_image_view, image_view];

                let create_info = vk::FramebufferCreateInfo::default()
                    .width(swap_chain.extent.width)
                    .height(swap_chain.extent.height)
                    .layers(1)
                    .attachments(&attachments)
                    .render_pass(render_pass);

                device
                    .device
                    .create_framebuffer(&create_info, None)
                    .expect("failed to create framebuffer!")
            })
            .collect::<Vec<vk::Framebuffer>>()
    }

    unsafe fn begin_single_time_command(&self) -> vk::CommandBuffer {
        let device = &self.device.device;

        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.transient_command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let command_buffer = device
            .allocate_command_buffers(&allocate_info)
            .expect("failed to allocate transient command buffer!")[0];
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        device
            .begin_command_buffer(command_buffer, &begin_info)
            .expect("failed to begin single time command buffer!");

        command_buffer
    }

    unsafe fn end_single_time_command(&self, command_buffer: vk::CommandBuffer) {
        let device = &self.device.device;
        device
            .end_command_buffer(command_buffer)
            .expect("failed to end single time command buffer!");

        let command_buffers = [command_buffer];
        let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);

        device
            .queue_submit(
                self.device.graphic_queue.unwrap(),
                &[submit_info],
                vk::Fence::null(),
            )
            .expect("failed to submit single time command buffer");

        // todo: Schedule multiple transfers simultaneously and wait for all of them complete, instead of executing one at a time.
        device
            .device_wait_idle()
            .expect("failed to wait device idle!");
        device.free_command_buffers(self.transient_command_pool, &[command_buffer]);
    }

    fn has_stencil_component(format: vk::Format) -> bool {
        format == vk::Format::D32_SFLOAT_S8_UINT
            || format == vk::Format::D24_UNORM_S8_UINT
            || format == vk::Format::D16_UNORM_S8_UINT
    }

    unsafe fn find_depth_format(instance: &ash::Instance, device: &gpu::Device) -> vk::Format {
        Self::find_supported_format(
            instance,
            device,
            vec![
                vk::Format::D32_SFLOAT,
                vk::Format::D32_SFLOAT_S8_UINT,
                vk::Format::D24_UNORM_S8_UINT,
            ],
            vk::ImageTiling::OPTIMAL,
            vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
        )
    }

    unsafe fn find_supported_format(
        instance: &ash::Instance,
        device: &gpu::Device,
        candidates: Vec<vk::Format>,
        tiling: vk::ImageTiling,
        features: vk::FormatFeatureFlags,
    ) -> vk::Format {
        for format in candidates {
            let properties =
                instance.get_physical_device_format_properties(device.physical_device, format);
            if tiling == vk::ImageTiling::LINEAR
                && properties.linear_tiling_features & features == features
            {
                return format;
            } else if tiling == vk::ImageTiling::OPTIMAL
                && properties.optimal_tiling_features & features == features
            {
                return format;
            }
        }

        panic!("failed to find supported format!")
    }
}

impl Drop for ForwardRenderer {
    fn drop(&mut self) {
        unsafe {
            let device = &self.device.device;
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

            self.image_available_semaphores
                .iter()
                .for_each(|&semaphore| device.destroy_semaphore(semaphore, None));
            self.render_finished_semaphores
                .iter()
                .for_each(|&semaphore| device.destroy_semaphore(semaphore, None));
            self.in_flight_fences
                .iter()
                .for_each(|&fence| device.destroy_fence(fence, None));

            device.destroy_command_pool(self.command_pool, None);
            device.destroy_command_pool(self.transient_command_pool, None);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}

use super::*;
use ash::vk;
use ash::vk::BufferCopy;
use std::ffi::c_void;
use std::mem::{align_of, size_of};
use std::rc::Rc;
use winit::window::Window;

pub struct GPU {
    pub context: VkContext,
    pub device_context: VkDeviceContext,
    pub swap_chain: SwapChain,

    transient_command_pool: vk::CommandPool,
}

impl GPU {
    pub fn new(window: Rc<Window>) -> Self {
        let context = VkContext::new(window);
        let device_context = VkDeviceContext::new(&context);
        let swap_chain = SwapChain::new(&context, &device_context);
        let transient_command_pool = Self::create_command_pools(&device_context);

        Self {
            context,
            device_context,
            swap_chain,
            transient_command_pool,
        }
    }

    pub fn create_shader_module(&self, code: &[u32]) -> vk::ShaderModule {
        unsafe {
            let create_info = vk::ShaderModuleCreateInfo::default().code(code);

            self.device_context
                .device
                .create_shader_module(&create_info, None)
                .expect("failed to create shader module!")
        }
    }

    pub fn create_descriptor_set_layout(
        &self,
        bindings: &Vec<vk::DescriptorSetLayoutBinding>,
    ) -> vk::DescriptorSetLayout {
        unsafe {
            let create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(bindings);
            self.device_context
                .device
                .create_descriptor_set_layout(&create_info, None)
                .expect("failed to create descriptor set layout!")
        }
    }

    pub fn create_descriptor_sets(
        &self,
        descriptor_pool: vk::DescriptorPool,
        layouts: &Vec<vk::DescriptorSetLayout>,
    ) -> Vec<vk::DescriptorSet> {
        unsafe {
            let allocate_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(layouts);

            let descriptor_sets = self
                .device_context
                .device
                .allocate_descriptor_sets(&allocate_info)
                .expect("failed to allocate descriptor sets!");

            descriptor_sets
        }
    }

    pub fn create_texture_image(
        &self,
        path: &str,
    ) -> (vk::Image, vk::DeviceMemory, vk::ImageView, vk::Sampler) {
        unsafe {
            let image = image::open(path).expect("failed to load image!");
            let image_rgba8 = image.to_rgba8();
            let width = image_rgba8.width();
            let height = image_rgba8.height();
            let mip_levels = ((width.min(height) as f32).log2().floor() + 1.0) as u32;
            let pixels = image_rgba8.into_raw();
            let image_size = (pixels.len() * size_of::<u8>()) as vk::DeviceSize;

            let (staging_buffer, staging_memory, _) = self.device_context.create_buffer(
                image_size,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
            );
            let staging_memory_mapped = self
                .device_context
                .device
                .map_memory(staging_memory, 0, image_size, vk::MemoryMapFlags::empty())
                .expect("failed to map staging memory!");

            let mut align = ash::util::Align::new(
                staging_memory_mapped,
                align_of::<u8>() as vk::DeviceSize,
                image_size,
            );
            align.copy_from_slice(&pixels);
            self.device_context.device.unmap_memory(staging_memory);

            let (image, memory) = self.device_context.create_image(
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
                self.transition_image_layout(
                    image,
                    vk::Format::R8G8B8A8_SRGB,
                    mip_levels,
                    vk::ImageLayout::UNDEFINED,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                );
                self.copy_buffer_to_image(staging_buffer, image, width, height);
                if mip_levels > 1 {
                    self.generate_mipmaps(
                        image,
                        vk::Format::R8G8B8A8_SRGB,
                        width,
                        height,
                        mip_levels,
                    );
                } else {
                    self.transition_image_layout(
                        image,
                        vk::Format::R8G8B8A8_SRGB,
                        1,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    );
                }

                self.device_context.device.free_memory(staging_memory, None);
                self.device_context
                    .device
                    .destroy_buffer(staging_buffer, None);
            }

            let image_view = self.device_context.create_image_view(
                image,
                vk::Format::R8G8B8A8_SRGB,
                vk::ImageAspectFlags::COLOR,
                mip_levels,
            );

            let create_info = vk::SamplerCreateInfo::default()
                .anisotropy_enable(true)
                .max_anisotropy(
                    self.device_context
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
                .device_context
                .device
                .create_sampler(&create_info, None)
                .expect("failed to create image sampler!");

            (image, memory, image_view, sampler)
        }
    }

    pub fn create_buffer_with_data<T: Copy>(
        &self,
        array: &Vec<T>,
        usage: vk::BufferUsageFlags,
    ) -> (vk::Buffer, vk::DeviceMemory) {
        unsafe {
            let buffer_size = (size_of::<T>() * array.len()) as vk::DeviceSize;
            let (staging_buffer, staging_memory, _) = self.device_context.create_buffer(
                buffer_size,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
            );

            let staging_memory_mapped = self
                .device_context
                .device
                .map_memory(staging_memory, 0, buffer_size, vk::MemoryMapFlags::empty())
                .expect("failed to map buffer staging memory!");
            let mut align = ash::util::Align::new(
                staging_memory_mapped,
                align_of::<T>() as vk::DeviceSize,
                buffer_size,
            );
            align.copy_from_slice(array);
            self.device_context.device.unmap_memory(staging_memory);

            let (buffer, buffer_memory, _) = self.device_context.create_buffer(
                buffer_size,
                vk::BufferUsageFlags::TRANSFER_DST | usage,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            );

            // The transfer of data to the GPU is an operation that happens in the background and the specification
            // simply tells us that it is guaranteed to be complete as of the next call to vkQueueSubmit.
            // https://registry.khronos.org/vulkan/specs/1.3-extensions/html/chap7.html#synchronization-submission-host-writes
            self.copy_buffer(staging_buffer, buffer, buffer_size);
            self.device_context
                .device
                .destroy_buffer(staging_buffer, None);
            self.device_context.device.free_memory(staging_memory, None);

            (buffer, buffer_memory)
        }
    }

    pub fn transition_image_layout(
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

        unsafe {
            // https://themaister.net/blog/2019/08/14/yet-another-blog-explaining-vulkan-synchronization/
            // 1. Wait for srcStageMask to complete
            // 2. Make all writes performed in possible combinations of srcStageMask + srcAccessMask available
            // 3. Make available memory visible to possible combinations of dstStageMask + dstAccessMask.
            // 4. Unblock work in dstStageMask.
            self.device_context.device.cmd_pipeline_barrier(
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
    }

    pub fn copy_buffer(
        &self,
        src_buffer: vk::Buffer,
        dst_buffer: vk::Buffer,
        size: vk::DeviceSize,
    ) {
        unsafe {
            let command_buffer = self.begin_single_time_command();
            let region = BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size,
            };
            self.device_context.device.cmd_copy_buffer(
                command_buffer,
                src_buffer,
                dst_buffer,
                &[region],
            );

            self.end_single_time_command(command_buffer);
        }
    }

    pub fn create_mapped_buffers(
        &self,
        size: vk::DeviceSize,
    ) -> (vk::Buffer, vk::DeviceMemory, *mut c_void) {
        unsafe {
            let (buffer, memory, _) = self.device_context.create_buffer(
                size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );

            let memory_mapped = self
                .device_context
                .device
                .map_memory(memory, 0, size, vk::MemoryMapFlags::empty())
                .expect("failed to map buffer memory!");

            (buffer, memory, memory_mapped)
        }
    }

    pub fn copy_buffer_to_image(
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

        unsafe {
            self.device_context.device.cmd_copy_buffer_to_image(
                command_buffer,
                buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            );
        }

        self.end_single_time_command(command_buffer);
    }

    pub fn generate_mipmaps(
        &self,
        image: vk::Image,
        format: vk::Format,
        width: u32,
        height: u32,
        mip_levels: u32,
    ) {
        let format_properties = self.get_format_properties(format);
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
            unsafe {
                self.device_context.device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[barrier],
                );
            }

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

            unsafe {
                self.device_context.device.cmd_blit_image(
                    command_buffer,
                    image,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[region],
                    vk::Filter::LINEAR,
                );
            }

            barrier.old_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
            barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_READ;
            barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;
            barrier.subresource_range.base_mip_level = i - 1;
            unsafe {
                self.device_context.device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::VERTEX_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[barrier],
                );
            }

            mip_width = next_mip_width;
            mip_height = next_mip_height;
        }

        barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
        barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
        barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
        barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;
        barrier.subresource_range.base_mip_level = mip_levels - 1;
        unsafe {
            self.device_context.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::VERTEX_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }

        self.end_single_time_command(command_buffer);
    }

    pub fn find_supported_format(
        &self,
        candidates: Vec<vk::Format>,
        tiling: vk::ImageTiling,
        features: vk::FormatFeatureFlags,
    ) -> vk::Format {
        for format in candidates {
            let properties = self.get_format_properties(format);
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

    fn get_format_properties(&self, format: vk::Format) -> vk::FormatProperties {
        unsafe {
            self.context
                .instance
                .get_physical_device_format_properties(self.device_context.physical_device, format)
        }
    }

    fn create_command_pools(device: &VkDeviceContext) -> vk::CommandPool {
        // VK_COMMAND_POOL_CREATE_TRANSIENT_BIT:
        //   Hint that command buffers are rerecorded with new commands very often (may change memory allocation behavior)
        // VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT:
        //   Allow command buffers to be rerecorded individually, without this flag they all have to be reset together
        let create_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::TRANSIENT)
            .queue_family_index(device.graphic_queue_family.unwrap());

        unsafe {
            let transient_command_pool = device
                .device
                .create_command_pool(&create_info, None)
                .expect("failed to create transient command pool!");

            transient_command_pool
        }
    }

    fn begin_single_time_command(&self) -> vk::CommandBuffer {
        unsafe {
            let device = &self.device_context.device;

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
    }

    fn end_single_time_command(&self, command_buffer: vk::CommandBuffer) {
        unsafe {
            let device = &self.device_context.device;
            device
                .end_command_buffer(command_buffer)
                .expect("failed to end single time command buffer!");

            let command_buffers = [command_buffer];
            let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);

            device
                .queue_submit(
                    self.device_context.graphic_queue.unwrap(),
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
    }

    fn has_stencil_component(format: vk::Format) -> bool {
        format == vk::Format::D32_SFLOAT_S8_UINT
            || format == vk::Format::D24_UNORM_S8_UINT
            || format == vk::Format::D16_UNORM_S8_UINT
    }
}

impl Drop for GPU {
    fn drop(&mut self) {
        unsafe {
            let device = &self.device_context.device;
            device.device_wait_idle().unwrap();

            for &image_view in self.swap_chain.image_views.iter() {
                device.destroy_image_view(image_view, None);
            }
            self.swap_chain
                .swap_chain_fn
                .as_ref()
                .unwrap()
                .destroy_swapchain(self.swap_chain.swap_chain.unwrap(), None);

            device.destroy_command_pool(self.transient_command_pool, None);

            device.destroy_device(None);

            let context = &self.context;
            context
                .surface_fn
                .as_ref()
                .unwrap()
                .destroy_surface(context.surface.unwrap(), None);
            if context.debug_utils_fn.is_some() && context.debug_utils_messenger.is_some() {
                context
                    .debug_utils_fn
                    .as_ref()
                    .unwrap()
                    .destroy_debug_utils_messenger(context.debug_utils_messenger.unwrap(), None);
            }
            context.instance.destroy_instance(None);
        }
    }
}

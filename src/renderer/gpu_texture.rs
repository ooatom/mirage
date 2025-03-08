use crate::assets::Texture;
use crate::gpu::GPU;
use ash::vk;

#[derive(Debug, Copy, Clone)]
pub struct GPUTexture {
    pub image: vk::Image,
    pub image_memory: vk::DeviceMemory,
    pub image_view: vk::ImageView,
    pub image_sampler: vk::Sampler,
}

impl GPUTexture {
    pub fn new(gpu: &GPU, texture: &Texture) -> Self {
        unsafe {
            let width = texture.width;
            let height = texture.height;
            let mip_levels = texture.mip_levels;
            let pixels = &texture.pixels;
            let image_size = (pixels.len() * size_of::<u8>()) as vk::DeviceSize;

            let (staging_buffer, staging_memory, _) = gpu.device_context.create_buffer(
                image_size,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
            );
            let staging_memory_mapped = gpu
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
            gpu.device_context.device.unmap_memory(staging_memory);

            let (image, image_memory) = gpu.device_context.create_image(
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
                gpu.transition_image_layout(
                    image,
                    vk::Format::R8G8B8A8_SRGB,
                    mip_levels,
                    vk::ImageLayout::UNDEFINED,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                );
                gpu.copy_buffer_to_image(staging_buffer, image, width, height);
                if mip_levels > 1 {
                    gpu.generate_mipmaps(
                        image,
                        vk::Format::R8G8B8A8_SRGB,
                        width,
                        height,
                        mip_levels,
                    );
                } else {
                    gpu.transition_image_layout(
                        image,
                        vk::Format::R8G8B8A8_SRGB,
                        1,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    );
                }

                gpu.device_context.device.free_memory(staging_memory, None);
                gpu.device_context
                    .device
                    .destroy_buffer(staging_buffer, None);
            }

            let image_view = gpu.device_context.create_image_view(
                image,
                vk::Format::R8G8B8A8_SRGB,
                vk::ImageAspectFlags::COLOR,
                mip_levels,
            );

            let create_info = vk::SamplerCreateInfo::default()
                .anisotropy_enable(true)
                .max_anisotropy(
                    gpu.device_context
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

            let image_sampler = gpu
                .device_context
                .device
                .create_sampler(&create_info, None)
                .expect("failed to create image sampler!");

            Self {
                image,
                image_memory,
                image_view,
                image_sampler,
            }
        }
    }

    pub fn drop(&mut self, gpu: &GPU) {
        unsafe {}
    }
}

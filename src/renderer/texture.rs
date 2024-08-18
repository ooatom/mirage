use crate::gpu::GPU;
use ash::vk;

#[derive(Copy, Clone)]
pub struct Texture {
    pub image: vk::Image,
    pub image_memory: vk::DeviceMemory,
    pub image_view: vk::ImageView,
    pub image_sampler: vk::Sampler,
}

impl Texture {
    pub fn load(gpu: &GPU, path: &str) -> Self {
        let (image, image_memory, image_view, image_sampler) = gpu.create_texture_image(path);

        Self {
            image,
            image_memory,
            image_view,
            image_sampler,
        }
    }
}

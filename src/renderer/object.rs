use crate::math::{Mat4, Vec3};
use crate::renderer::{Geom, Shading, GPU};
use ash::vk;

pub struct Object {
    pub geom: Geom,
    pub shading: Shading,
    pub model: Mat4,

    pub texture_image: vk::Image,
    pub texture_image_memory: vk::DeviceMemory,
    pub texture_image_view: vk::ImageView,
    pub texture_image_sampler: vk::Sampler,
}

impl Object {
    pub fn new(gpu: &GPU, geom: Geom, shading: Shading) -> Self {
        let (texture_image, texture_image_memory, texture_image_view, texture_image_sampler) =
            gpu.create_texture_image("assets/viking_room.png");

        Self {
            geom,
            shading,
            model: Mat4::identity(),

            texture_image,
            texture_image_memory,
            texture_image_view,
            texture_image_sampler,
        }
    }

    pub fn update(&mut self) {
        self.model =
            Mat4::translate(Vec3::new(0.0, 0.0, -0.9)) * Mat4::scale(Vec3::new(5.0, 5.0, 5.0));
    }

    pub fn render(&self, gpu: &GPU, command_buffer: vk::CommandBuffer, frame_index: usize) {
        unsafe {
            let device = &gpu.device_context.device;

            {
                let image_infos = [vk::DescriptorImageInfo {
                    image_view: self.texture_image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    sampler: self.texture_image_sampler,
                }];

                let texture_write = vk::WriteDescriptorSet::default()
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&image_infos)
                    .dst_set(self.shading.descriptor_sets[frame_index])
                    .dst_binding(0)
                    .dst_array_element(0);

                let sampler_write = vk::WriteDescriptorSet::default()
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .image_info(&image_infos)
                    .dst_set(self.shading.descriptor_sets[frame_index])
                    .dst_binding(1)
                    .dst_array_element(0);

                device.update_descriptor_sets(&[texture_write, sampler_write], &[]);
            }

            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.shading.pipeline,
            );
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[self.geom.vertex_buffer], &[0]);
            device.cmd_bind_index_buffer(
                command_buffer,
                self.geom.index_buffer,
                0,
                vk::IndexType::UINT32,
            );
            // device.cmd_push_constants();
            device.cmd_draw_indexed(command_buffer, self.geom.indices.len() as u32, 1, 0, 0, 0);
        }
    }
}

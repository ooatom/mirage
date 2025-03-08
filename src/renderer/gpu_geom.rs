use crate::assets::Geom;
use crate::gpu::GPU;
use ash::vk;

#[derive(Debug, Copy, Clone)]
pub struct GPUGeom {
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,
    pub indices_length: usize,
}

impl GPUGeom {
    pub fn new(gpu: &GPU, geom: &Geom) -> Self {
        let (vertex_buffer, vertex_buffer_memory) =
            gpu.create_buffer_with_data(&geom.vertices, vk::BufferUsageFlags::VERTEX_BUFFER);
        let (index_buffer, index_buffer_memory) =
            gpu.create_buffer_with_data(&geom.indices, vk::BufferUsageFlags::INDEX_BUFFER);

        Self {
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer,
            index_buffer_memory,
            indices_length: geom.indices.len(),
        }
    }

    pub fn drop(&mut self, gpu: &GPU) {
        unsafe {
            let device = &gpu.device_context.device;
            device.destroy_buffer(self.vertex_buffer, None);
            device.free_memory(self.vertex_buffer_memory, None);
            device.destroy_buffer(self.index_buffer, None);
            device.free_memory(self.index_buffer_memory, None);
        }
    }
}

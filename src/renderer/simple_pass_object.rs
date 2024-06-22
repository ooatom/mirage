use super::*;
use crate::Assets;
use ash::vk;
use std::io::Cursor;
use tobj::LoadError;

pub struct SimplePassObject {
    pub vertices: Vec<simple_pass::Vertex>,
    pub indices: Vec<u32>,

    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,

    pub texture_image: vk::Image,
    pub texture_image_memory: vk::DeviceMemory,
    pub texture_image_view: vk::ImageView,
    pub texture_image_sampler: vk::Sampler,
}

impl SimplePassObject {
    pub fn new(pass: &SimplePass) -> Self {
        unsafe {
            let (texture_image, texture_image_memory, texture_image_view, texture_image_sampler) =
                pass.create_texture_image("assets/texture.jpg");
            let (vertices, indices) = SimplePassObject::load_model();
            // let (vertices, indices) = SimplePassObject::create_simple_data();
            let (vertex_buffer, vertex_buffer_memory) =
                pass.create_buffer_with_data(&vertices, vk::BufferUsageFlags::VERTEX_BUFFER);
            let (index_buffer, index_buffer_memory) =
                pass.create_buffer_with_data(&indices, vk::BufferUsageFlags::INDEX_BUFFER);

            Self {
                vertices,
                indices,

                vertex_buffer,
                vertex_buffer_memory,
                index_buffer,
                index_buffer_memory,

                texture_image,
                texture_image_memory,
                texture_image_view,
                texture_image_sampler,
            }
        }
    }

    #[allow(dead_code)]
    fn load_model() -> (Vec<simple_pass::Vertex>, Vec<u32>) {
        let mut buffer = Cursor::new(Assets::get("test_2d.obj").unwrap().data);
        let (models, _) = tobj::load_obj_buf(&mut buffer, &tobj::GPU_LOAD_OPTIONS, |mat_path| {
            let path = Assets::get(mat_path.to_str().unwrap());
            if let Some(file) = path {
                let mut buffer = Cursor::new(file.data);
                return tobj::load_mtl_buf(&mut buffer);
            }

            // #[cfg(feature = "log")]
            // log::error!("load_mtl - failed to open {:?} due to {}", file_name, _e);
            Err(LoadError::OpenFileFailed)
        })
        .expect("failed to load obj!");

        let mesh = &models[0].mesh;
        let vertex_count = mesh.positions.len() / 3;
        let mut vertices = Vec::with_capacity(vertex_count);

        for i in 0..vertex_count {
            let vertex = simple_pass::Vertex {
                position: [
                    mesh.positions[i * 3],
                    mesh.positions[i * 3 + 1],
                    mesh.positions[i * 3 + 2],
                ],
                color: [1.0, 1.0, 1.0],
                uv: [mesh.texcoords[i * 2], 1.0 - mesh.texcoords[i * 2 + 1]],
            };
            vertices.push(vertex);
        }

        let indices = mesh.indices.to_vec();

        (vertices, indices)
    }

    #[allow(dead_code)]
    fn create_simple_data() -> (Vec<simple_pass::Vertex>, Vec<u32>) {
        let indices = vec![0, 1, 2, 0, 2, 3];

        let vertices = [
            [-0.5, -0.5, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            [-0.5, 0.5, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0],
            [0.5, 0.5, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0],
            [0.5, -0.5, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0],
        ]
        .map(|data| simple_pass::Vertex {
            position: [data[0], data[1], data[2]],
            color: [data[3], data[4], data[5]],
            uv: [data[6], data[7]],
        })
        .to_vec();

        (vertices, indices)
    }
}

use crate::assets::asset_impl::AssetImpl;
use crate::assets::Assets;
use crate::scene::vertex::Vertex;
use std::io::Cursor;
use tobj::LoadError;

#[derive(Debug, Clone)]
pub struct Geom {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Geom {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        Self { vertices, indices }
    }
}

impl Default for Geom {
    fn default() -> Self {
        let indices = vec![0, 1, 2, 0, 2, 3];

        let vertices = [
            [-0.5, 0.5, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            [-0.5, -0.5, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
            [0.5, -0.5, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            [0.5, 0.5, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0],
        ]
        .map(|data| Vertex {
            position: [data[0], data[1], data[2]],
            color: [data[3], data[4], data[5]],
            uv: [data[6], data[7]],
        })
        .to_vec();

        Self::new(vertices, indices)
    }
}

impl AssetImpl for Geom {
    fn load(data: &[u8]) -> Option<Self> {
        let mut buffer = Cursor::new(data);
        let (models, _) = tobj::load_obj_buf(&mut buffer, &tobj::GPU_LOAD_OPTIONS, |mat_path| {
            if let Some(file) = Assets::load_raw(mat_path.to_str().unwrap()) {
                let mut buffer = Cursor::new(file);
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
            let vertex = Vertex {
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

        Some(Self::new(vertices, indices))
    }
}

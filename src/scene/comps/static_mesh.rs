use crate::scene::ecs::Comp;
use ash::vk;
use crate::renderer::{Geom, Material};

#[derive(Debug, Copy, Clone)]
pub struct StaticMesh {
    pub polygon_mode: vk::PolygonMode,

    pub topology: vk::PrimitiveTopology,
    pub vertex_count: usize,
    pub geom: Geom,
    pub material: Material,
}

impl Comp for StaticMesh {}

impl StaticMesh {
    pub fn new(geom: Geom, material: Material) -> Self {
        Self {
            polygon_mode: vk::PolygonMode::FILL,
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            vertex_count: geom.indices_len,
            geom,
            material
        }
    }
}

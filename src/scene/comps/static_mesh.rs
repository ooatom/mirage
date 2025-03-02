use crate::assets::{AssetHandle, Geom, Material};
use crate::scene::ecs::Comp;
use ash::vk;

#[derive(Debug, Clone)]
pub struct StaticMesh {
    pub polygon_mode: vk::PolygonMode,
    pub topology: vk::PrimitiveTopology,
    pub geom: Option<AssetHandle<Geom>>,
    pub material: Option<AssetHandle<Material>>,
}

impl Comp for StaticMesh {}

impl StaticMesh {
    pub fn new(geom: Option<AssetHandle<Geom>>, material: Option<AssetHandle<Material>>) -> Self {
        Self {
            polygon_mode: vk::PolygonMode::FILL,
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            geom,
            material,
        }
    }
}

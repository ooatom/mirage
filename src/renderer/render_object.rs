use crate::assets::*;
use crate::math::Mat4;
use crate::renderer::GPUAssets;
use std::cell::RefCell;
use std::rc::Rc;

pub struct RenderObject {
    pub geom: AssetHandle<Geom>,
    pub material: AssetHandle<Material>,
    pub model: Mat4,
}

impl RenderObject {
    pub fn new(geom: AssetHandle<Geom>, material: AssetHandle<Material>, model: Mat4) -> Self {
        Self {
            geom,
            material,
            model,
        }
    }
}

pub struct RenderContext {
    pub gpu_assets: Rc<RefCell<GPUAssets>>,
    pub objects: Vec<RenderObject>,
}

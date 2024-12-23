use super::*;
use crate::math::{Mat4};

pub struct RenderObject {
    pub geom: Geom,
    pub material: Material,
    pub model: Mat4,
}

impl RenderObject {
    pub fn new(geom: Geom, material: Material, model: Mat4) -> Self {
        Self {
            geom,
            material,
            model,
        }
    }
}

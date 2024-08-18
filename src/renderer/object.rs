use super::*;
use crate::math::{Mat4};

pub struct Object {
    pub geom: Geom,
    pub material: Material,
    pub model: Mat4,
}

impl Object {
    pub fn new(geom: Geom, material: Material) -> Self {
        Self {
            geom,
            material,
            model: Mat4::identity(),
        }
    }
}

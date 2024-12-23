use crate::math::{Euler, Mat4, Vec3};
use crate::scene::ecs::*;
use egui::ahash::HashMapExt;
use std::cell::RefCell;

#[derive(Debug)]
pub struct Transform {
    pub location: Vec3,
    pub rotation: Euler,
    pub scale: Vec3,
    matrix_key: RefCell<Option<[f32; 10]>>,
    matrix_cache: RefCell<Mat4>,
}
impl Comp for Transform {}

impl Transform {
    pub fn new(location: Vec3, rotation: Euler, scale: Vec3) -> Self {
        Self {
            location,
            rotation,
            scale,
            matrix_key: RefCell::new(None),
            matrix_cache: RefCell::new(Mat4::default()),
        }
    }

    pub fn matrix(&self) -> Mat4 {
        if self.update_matrix_key() {
            *self.matrix_cache.borrow_mut() = Mat4::compose(self.location, self.rotation, self.scale);
        }
        self.matrix_cache.borrow().clone()
    }

    pub fn matrix_mut(&mut self, mat4: Mat4) {
        let (location, rotation, scale) = Mat4::decompose(mat4);
        self.location = location;
        self.rotation = rotation;
        self.scale = scale;
        *self.matrix_cache.borrow_mut() = mat4;

        self.update_matrix_key();
    }

    fn update_matrix_key(&self) -> bool {
        let curr_key = [
            self.location.x,
            self.location.y,
            self.location.z,
            self.rotation.x,
            self.rotation.y,
            self.rotation.z,
            self.rotation.order as u8 as f32,
            self.scale.x,
            self.scale.y,
            self.scale.z,
        ];

        let mut maybe_key = self.matrix_key.borrow_mut();
        match *maybe_key {
            Some(key) if key.eq(&curr_key) => false,
            _ => {
                *maybe_key = Some(curr_key);
                true
            }
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::new(Vec3::zero(), Euler::default(), Vec3::one())
    }
}

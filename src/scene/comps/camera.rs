use crate::scene::Comp;

pub struct Camera {
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
}

impl Comp for Camera {}
impl Camera {
    pub fn new(fov: f32, aspect: f32, near: f32) -> Camera {
        Self { fov, aspect, near }
    }
}

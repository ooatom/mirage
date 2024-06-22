pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub s: f32,
}

impl Quat {
    #[inline]
    pub fn new(x: f32, y: f32, z: f32, s: f32) -> Self {
        Self { x, y, z, s }
    }
}

impl Default for Quat {
    #[inline]
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            s: 1.0,
        }
    }
}

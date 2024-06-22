use crate::math::{Mat4, Quat};

pub enum EulerOrder {
    ZYX,
}

pub struct Euler {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub order: EulerOrder,
}

impl Euler {
    #[inline]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            x,
            y,
            z,
            order: EulerOrder::ZYX,
        }
    }
}

impl Default for Euler {
    #[inline]
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            order: EulerOrder::ZYX,
        }
    }
}

impl From<Quat> for Euler {
    fn from(value: Quat) -> Self {
        Euler::new(value.x, value.y, value.z)
    }
}

impl From<Mat4> for Euler {
    fn from(value: Mat4) -> Self {
        let y = (-value.c0.z.clamp(-1.0, 1.0)).asin();

        if value.c0.z.abs() < 1.0 {
            let x = value.c1.z.atan2(value.c2.z);
            let z = value.c0.y.atan2(value.c0.x);
            Euler::new(x, y, z)
        } else {
            let x = 0.0;
            let z = value.c2.y.atan2(value.c2.x);
            Euler::new(x, y, z)
        }
    }
}

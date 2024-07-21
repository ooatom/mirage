use crate::math::{Vec2, Vec3};
use std::ops::{Add, Div, Mul, Neg, Sub};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    #[inline]
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    #[inline]
    pub fn dot(&self, v: Self) -> f32 {
        self.x * v.x + self.y * v.y + self.z * v.z + self.w * v.w
    }

    #[inline]
    pub fn normalize(&self) -> Self {
        let denominator = 1.0 / self.len_sq().sqrt();
        Self {
            x: self.x * denominator,
            y: self.y * denominator,
            z: self.z * denominator,
            w: self.w * denominator,
        }
    }

    #[inline]
    pub fn len(&self) -> f32 {
        self.len_sq().sqrt()
    }

    #[inline]
    pub fn len_sq(&self) -> f32 {
        self.dot(*self)
    }
}

impl Default for Vec4 {
    #[inline]
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
        }
    }
}

impl From<[f32; 2]> for Vec4 {
    fn from(value: [f32; 2]) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: 0.0,
            w: 0.0,
        }
    }
}

impl From<[f32; 3]> for Vec4 {
    fn from(value: [f32; 3]) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: value[2],
            w: 0.0,
        }
    }
}

impl From<[f32; 4]> for Vec4 {
    fn from(value: [f32; 4]) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: value[2],
            w: value[3],
        }
    }
}

impl From<Vec2> for Vec4 {
    #[inline]
    fn from(value: Vec2) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: 0.0,
            w: 0.0,
        }
    }
}

impl From<Vec3> for Vec4 {
    #[inline]
    fn from(value: Vec3) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
            w: 0.0,
        }
    }
}

impl Add<Vec4> for Vec4 {
    type Output = Vec4;

    #[inline]
    fn add(self, rhs: Vec4) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
            w: self.w + rhs.w,
        }
    }
}

impl Sub<Vec4> for Vec4 {
    type Output = Vec4;

    fn sub(self, rhs: Vec4) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
            w: self.w - rhs.w,
        }
    }
}

impl Mul<Vec4> for Vec4 {
    type Output = Vec4;

    #[inline]
    fn mul(self, rhs: Vec4) -> Self::Output {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
            w: self.w * rhs.w,
        }
    }
}

impl Div<Vec4> for Vec4 {
    type Output = Vec4;

    #[inline]
    fn div(self, rhs: Vec4) -> Self::Output {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            z: self.z / rhs.z,
            w: self.w / rhs.w,
        }
    }
}

impl Add<f32> for Vec4 {
    type Output = Vec4;

    #[inline]
    fn add(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x + rhs,
            y: self.y + rhs,
            z: self.z + rhs,
            w: self.w + rhs,
        }
    }
}

impl Sub<f32> for Vec4 {
    type Output = Vec4;

    #[inline]
    fn sub(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x - rhs,
            y: self.y - rhs,
            z: self.z - rhs,
            w: self.w - rhs,
        }
    }
}

impl Mul<f32> for Vec4 {
    type Output = Vec4;

    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
            w: self.w * rhs,
        }
    }
}

impl Div<f32> for Vec4 {
    type Output = Vec4;

    #[inline]
    fn div(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
            w: self.w / rhs,
        }
    }
}

impl Neg for Vec4 {
    type Output = Vec4;

    #[inline]
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: -self.w,
        }
    }
}

impl Add<Vec4> for f32 {
    type Output = Vec4;

    #[inline]
    fn add(self, rhs: Vec4) -> Self::Output {
        Vec4 {
            x: self + rhs.x,
            y: self + rhs.y,
            z: self + rhs.z,
            w: self + rhs.w,
        }
    }
}

impl Sub<Vec4> for f32 {
    type Output = Vec4;

    #[inline]
    fn sub(self, rhs: Vec4) -> Self::Output {
        Vec4 {
            x: self - rhs.x,
            y: self - rhs.y,
            z: self - rhs.z,
            w: self - rhs.w,
        }
    }
}

impl Mul<Vec4> for f32 {
    type Output = Vec4;

    #[inline]
    fn mul(self, rhs: Vec4) -> Self::Output {
        Vec4 {
            x: self * rhs.x,
            y: self * rhs.y,
            z: self * rhs.z,
            w: self * rhs.w,
        }
    }
}

impl Div<Vec4> for f32 {
    type Output = Vec4;

    #[inline]
    fn div(self, rhs: Vec4) -> Self::Output {
        Vec4 {
            x: self / rhs.x,
            y: self / rhs.y,
            z: self / rhs.z,
            w: self / rhs.w,
        }
    }
}

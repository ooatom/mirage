use crate::math::{Vec2, Vec4};
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::usize::MAX;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    #[inline]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn dot(&self, v: Self) -> f32 {
        self.x * v.x + self.y * v.y + self.z * v.z
    }

    #[inline]
    pub fn cross(&self, rhs: Self) -> Self {
        Self {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }

    #[inline]
    pub fn normalize(&self) -> Self {
        let denominator = 1.0 / self.len_sq().sqrt();
        Self {
            x: self.x * denominator,
            y: self.y * denominator,
            z: self.z * denominator,
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

impl Default for Vec3 {
    #[inline]
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

impl From<[f32; 2]> for Vec3 {
    fn from(value: [f32; 2]) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: 0.0,
        }
    }
}

impl From<[f32; 3]> for Vec3 {
    fn from(value: [f32; 3]) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: value[2],
        }
    }
}

impl From<[f32; 4]> for Vec3 {
    fn from(value: [f32; 4]) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: value[2],
        }
    }
}

impl From<Vec2> for Vec3 {
    #[inline]
    fn from(value: Vec2) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: 0.0,
        }
    }
}

impl From<Vec4> for Vec3 {
    #[inline]
    fn from(value: Vec4) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

impl Add<Vec3> for Vec3 {
    type Output = Vec3;

    #[inline]
    fn add(self, rhs: Vec3) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl Sub<Vec3> for Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: Vec3) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl Mul<Vec3> for Vec3 {
    type Output = Vec3;

    #[inline]
    fn mul(self, rhs: Vec3) -> Self::Output {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
        }
    }
}

impl Div<Vec3> for Vec3 {
    type Output = Vec3;

    #[inline]
    fn div(self, rhs: Vec3) -> Self::Output {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            z: self.z / rhs.z,
        }
    }
}

impl Add<f32> for Vec3 {
    type Output = Vec3;

    #[inline]
    fn add(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x + rhs,
            y: self.y + rhs,
            z: self.z + rhs,
        }
    }
}

impl Sub<f32> for Vec3 {
    type Output = Vec3;

    #[inline]
    fn sub(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x - rhs,
            y: self.y - rhs,
            z: self.z - rhs,
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;

    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl Div<f32> for Vec3 {
    type Output = Vec3;

    #[inline]
    fn div(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}

impl Neg for Vec3 {
    type Output = Vec3;

    #[inline]
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl Add<Vec3> for f32 {
    type Output = Vec3;

    #[inline]
    fn add(self, rhs: Vec3) -> Self::Output {
        Vec3 {
            x: self + rhs.x,
            y: self + rhs.y,
            z: self + rhs.z,
        }
    }
}

impl Sub<Vec3> for f32 {
    type Output = Vec3;

    #[inline]
    fn sub(self, rhs: Vec3) -> Self::Output {
        Vec3 {
            x: self - rhs.x,
            y: self - rhs.y,
            z: self - rhs.z,
        }
    }
}

impl Mul<Vec3> for f32 {
    type Output = Vec3;

    #[inline]
    fn mul(self, rhs: Vec3) -> Self::Output {
        Vec3 {
            x: self * rhs.x,
            y: self * rhs.y,
            z: self * rhs.z,
        }
    }
}

impl Div<Vec3> for f32 {
    type Output = Vec3;

    #[inline]
    fn div(self, rhs: Vec3) -> Self::Output {
        Vec3 {
            x: self / rhs.x,
            y: self / rhs.y,
            z: self / rhs.z,
        }
    }
}

use crate::math::{Vec3, Vec4};
use std::ops::{Add, Div, Mul, Neg, Sub};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn dot(&self, v: Self) -> f32 {
        self.x * v.x + self.y * v.y
    }

    #[inline]
    pub fn cross(&self, v: Self) -> f32 {
        self.x * v.y - self.y * v.x
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

impl Default for Vec2 {
    #[inline]
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

impl From<Vec3> for Vec2 {
    #[inline]
    fn from(value: Vec3) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

impl From<Vec4> for Vec2 {
    #[inline]
    fn from(value: Vec4) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

impl Add<Vec2> for Vec2 {
    type Output = Vec2;

    #[inline]
    fn add(self, rhs: Vec2) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub<Vec2> for Vec2 {
    type Output = Vec2;

    fn sub(self, rhs: Vec2) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Mul<Vec2> for Vec2 {
    type Output = Vec2;

    #[inline]
    fn mul(self, rhs: Vec2) -> Self::Output {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}

impl Div<Vec2> for Vec2 {
    type Output = Vec2;

    #[inline]
    fn div(self, rhs: Vec2) -> Self::Output {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
        }
    }
}

impl Add<f32> for Vec2 {
    type Output = Vec2;

    #[inline]
    fn add(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x + rhs,
            y: self.y + rhs,
        }
    }
}

impl Sub<f32> for Vec2 {
    type Output = Vec2;

    #[inline]
    fn sub(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x - rhs,
            y: self.y - rhs,
        }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;

    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Div<f32> for Vec2 {
    type Output = Vec2;

    #[inline]
    fn div(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl Neg for Vec2 {
    type Output = Vec2;

    #[inline]
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl Add<Vec2> for f32 {
    type Output = Vec2;

    #[inline]
    fn add(self, rhs: Vec2) -> Self::Output {
        Vec2 {
            x: self + rhs.x,
            y: self + rhs.y,
        }
    }
}

impl Sub<Vec2> for f32 {
    type Output = Vec2;

    #[inline]
    fn sub(self, rhs: Vec2) -> Self::Output {
        Vec2 {
            x: self - rhs.x,
            y: self - rhs.y,
        }
    }
}

impl Mul<Vec2> for f32 {
    type Output = Vec2;

    #[inline]
    fn mul(self, rhs: Vec2) -> Self::Output {
        Vec2 {
            x: self * rhs.x,
            y: self * rhs.y,
        }
    }
}

impl Div<Vec2> for f32 {
    type Output = Vec2;

    #[inline]
    fn div(self, rhs: Vec2) -> Self::Output {
        Vec2 {
            x: self / rhs.x,
            y: self / rhs.y,
        }
    }
}

use super::{Mat3, Mat4, Vec2};
use std::mem;
use std::ops::{Add, Div, Index, IndexMut, Mul, Neg, Sub};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Mat2 {
    pub c0: Vec2,
    pub c1: Vec2,
}

impl Mat2 {
    #[inline]
    pub fn new(c0r0: f32, c0r1: f32, c1r0: f32, c1r1: f32) -> Self {
        Self {
            c0: Vec2::new(c0r0, c0r1),
            c1: Vec2::new(c1r0, c1r1),
        }
    }

    #[inline]
    pub fn identity() -> Self {
        Self {
            c0: Vec2::new(1.0, 0.0),
            c1: Vec2::new(0.0, 1.0),
        }
    }

    #[inline]
    pub fn from_rows(row0: Vec2, row1: Vec2) -> Mat2 {
        Self {
            c0: Vec2::new(row0.x, row1.x),
            c1: Vec2::new(row0.y, row1.y),
        }
    }

    #[inline]
    pub fn from_cols(col0: Vec2, col1: Vec2) -> Mat2 {
        Self {
            c0: Vec2::new(col0.x, col0.y),
            c1: Vec2::new(col1.x, col1.y),
        }
    }

    #[inline]
    fn row(&self, index: usize) -> Vec2 {
        Vec2::new(self[index], self[index + 2])
    }

    #[inline]
    pub fn invert(&mut self) -> &mut Self {
        let det = self.determinant();
        if det == 0.0 {
            return self;
        }

        let c0r0_cof = self.c1.y;
        let c0r1_cof = self.c1.x;
        let c1r0_cof = self.c0.y;
        let c1r1_cof = self.c0.x;

        let det_recip = 1.0 / det;
        self.c0.x = c0r0_cof * det_recip;
        self.c0.y = c1r0_cof * det_recip;
        self.c1.x = c0r1_cof * det_recip;
        self.c1.y = c1r1_cof * det_recip;
        self
    }

    #[inline]
    pub fn transpose(&mut self) -> &mut Self {
        (self.c0.y, self.c1.x) = (self.c1.x, self.c0.y);
        self
    }

    #[inline]
    pub fn determinant(&self) -> f32 {
        self.c0.x * self.c1.y - self.c1.x * self.c0.y
    }
}

impl Default for Mat2 {
    #[inline]
    fn default() -> Self {
        Self {
            c0: Vec2::default(),
            c1: Vec2::default(),
        }
    }
}

impl From<Mat3> for Mat2 {
    #[inline]
    fn from(value: Mat3) -> Self {
        Self {
            c0: Vec2::from(value.c0),
            c1: Vec2::from(value.c1),
        }
    }
}

impl From<Mat4> for Mat2 {
    #[inline]
    fn from(value: Mat4) -> Self {
        Self {
            c0: Vec2::from(value.col(0)),
            c1: Vec2::from(value.col(1)),
        }
    }
}

impl AsRef<[f32; 4]> for Mat2 {
    #[inline]
    fn as_ref(&self) -> &[f32; 4] {
        unsafe { mem::transmute(self) }
    }
}

impl AsMut<[f32; 4]> for Mat2 {
    #[inline]
    fn as_mut(&mut self) -> &mut [f32; 4] {
        unsafe { mem::transmute(self) }
    }
}

impl Index<usize> for Mat2 {
    type Output = f32;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl IndexMut<usize> for Mat2 {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.as_mut().index_mut(index)
    }
}

impl Add<Mat2> for Mat2 {
    type Output = Mat2;

    #[inline]
    fn add(self, rhs: Mat2) -> Self::Output {
        Self::from_cols(self.c0 + rhs.c0, self.c1 + rhs.c1)
    }
}

impl Sub<Mat2> for Mat2 {
    type Output = Mat2;

    #[inline]
    fn sub(self, rhs: Mat2) -> Self::Output {
        Self::from_cols(self.c0 - rhs.c0, self.c1 - rhs.c1)
    }
}

impl Mul<Mat2> for Mat2 {
    type Output = Mat2;

    #[inline]
    fn mul(self, rhs: Mat2) -> Self::Output {
        let r0 = self.row(0);
        let r1 = self.row(1);

        let c0 = rhs.c0;
        let c1 = rhs.c1;

        Self {
            c0: Vec2::new(r0.dot(c0), r1.dot(c0)),
            c1: Vec2::new(r0.dot(c1), r1.dot(c1)),
        }
    }
}

impl Div<Mat2> for Mat2 {
    type Output = Mat2;

    #[inline]
    fn div(self, rhs: Mat2) -> Self::Output {
        let r0 = self.row(0);
        let r1 = self.row(1);

        let c0 = 1.0 / rhs.c0;
        let c1 = 1.0 / rhs.c1;

        Self {
            c0: Vec2::new(r0.dot(c0), r1.dot(c0)),
            c1: Vec2::new(r0.dot(c1), r1.dot(c1)),
        }
    }
}

impl Neg for Mat2 {
    type Output = Mat2;

    #[inline]
    fn neg(self) -> Self::Output {
        Self::from_cols(-self.c0, -self.c1)
    }
}

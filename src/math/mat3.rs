use super::{Mat2, Mat4, Vec3};
use std::mem;
use std::ops::{Add, Div, Index, IndexMut, Mul, Neg, Sub};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Mat3 {
    pub c0: Vec3,
    pub c1: Vec3,
    pub c2: Vec3,
}

impl Mat3 {
    #[rustfmt::skip]
    #[inline]
    pub fn new(c0r0: f32, c0r1: f32, c0r2: f32, c1r0: f32, c1r1: f32, c1r2: f32, c2r0: f32, c2r1: f32, c2r2: f32) -> Self {
        Self {
            c0: Vec3::new(c0r0, c0r1, c0r2),
            c1: Vec3::new(c1r0, c1r1, c1r2),
            c2: Vec3::new(c2r0, c2r1, c2r2),
        }
    }

    #[inline]
    pub fn identity() -> Self {
        Self {
            c0: Vec3::new(1.0, 0.0, 0.0),
            c1: Vec3::new(0.0, 1.0, 0.0),
            c2: Vec3::new(0.0, 0.0, 1.0),
        }
    }

    #[inline]
    pub fn from_cols(col0: Vec3, col1: Vec3, col2: Vec3) -> Mat3 {
        Self {
            c0: Vec3::new(col0.x, col0.y, col0.z),
            c1: Vec3::new(col1.x, col1.y, col1.z),
            c2: Vec3::new(col2.x, col2.y, col2.z),
        }
    }

    #[inline]
    pub fn from_rows(row0: Vec3, row1: Vec3, row2: Vec3) -> Mat3 {
        Self {
            c0: Vec3::new(row0.x, row1.x, row2.x),
            c1: Vec3::new(row0.y, row1.y, row2.y),
            c2: Vec3::new(row0.z, row1.z, row2.z),
        }
    }

    #[inline]
    pub fn row(&self, index: usize) -> Vec3 {
        Vec3::new(self[index], self[index + 3], self[index + 6])
    }

    #[inline]
    pub fn invert(&mut self) -> &mut Self {
        let c0r0_cof = self.c1.y * self.c2.z - self.c1.z * self.c2.y;
        let c0r1_cof = self.c1.x * self.c2.z - self.c1.z * self.c2.x;
        let c0r2_cof = self.c1.x * self.c2.y - self.c1.y * self.c2.x;

        let det = self.c0.x * c0r0_cof + self.c0.y * c0r1_cof + self.c0.z * c0r2_cof;
        if det == 0.0 {
            return self;
        }

        let c1r0_cof = self.c2.y * self.c0.z - self.c2.z * self.c0.y;
        let c1r1_cof = self.c2.z * self.c0.x - self.c2.x * self.c0.z;
        let c1r2_cof = self.c2.x * self.c0.y - self.c2.y * self.c0.x;
        let c2r0_cof = self.c0.y * self.c1.z - self.c0.z * self.c1.y;
        let c2r1_cof = self.c0.z * self.c1.x - self.c0.x * self.c1.z;
        let c2r2_cof = self.c0.x * self.c1.y - self.c0.y * self.c1.x;

        let det_recip = 1.0 / det;
        self.c0.x = c0r0_cof * det_recip;
        self.c0.y = c1r0_cof * det_recip;
        self.c0.z = c2r0_cof * det_recip;
        self.c1.x = c0r1_cof * det_recip;
        self.c1.y = c1r1_cof * det_recip;
        self.c1.z = c2r1_cof * det_recip;
        self.c2.x = c0r2_cof * det_recip;
        self.c2.y = c1r2_cof * det_recip;
        self.c2.z = c2r2_cof * det_recip;
        self
    }

    #[inline]
    pub fn transpose(&mut self) -> &mut Self {
        (self.c0.y, self.c1.x) = (self.c1.x, self.c0.y);
        self
    }

    #[inline]
    pub fn determinant(&self) -> f32 {
        let c0r0_cof = self.c1.y * self.c2.z - self.c1.z * self.c2.y;
        let c0r1_cof = self.c1.z * self.c2.x - self.c1.x * self.c2.z;
        let c0r2_cof = self.c1.x * self.c2.y - self.c1.y * self.c2.x;

        self.c0.x * c0r0_cof + self.c0.y * c0r1_cof + self.c0.z * c0r2_cof
    }
}

impl Default for Mat3 {
    #[inline]
    fn default() -> Self {
        Self {
            c0: Vec3::default(),
            c1: Vec3::default(),
            c2: Vec3::default(),
        }
    }
}

impl From<Mat2> for Mat3 {
    #[inline]
    fn from(value: Mat2) -> Self {
        Self {
            c0: Vec3::from(value.c0),
            c1: Vec3::from(value.c1),
            c2: Vec3::new(0.0, 0.0, 1.0),
        }
    }
}

impl From<Mat4> for Mat3 {
    #[inline]
    fn from(value: Mat4) -> Self {
        Self {
            c0: Vec3::from(value.c0),
            c1: Vec3::from(value.c1),
            c2: Vec3::from(value.c2),
        }
    }
}

impl AsRef<[f32; 9]> for Mat3 {
    #[inline]
    fn as_ref(&self) -> &[f32; 9] {
        unsafe { mem::transmute(self) }
    }
}

impl AsMut<[f32; 9]> for Mat3 {
    #[inline]
    fn as_mut(&mut self) -> &mut [f32; 9] {
        unsafe { mem::transmute(self) }
    }
}

impl Index<usize> for Mat3 {
    type Output = f32;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl IndexMut<usize> for Mat3 {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.as_mut().index_mut(index)
    }
}

impl Add<Mat3> for Mat3 {
    type Output = Mat3;

    #[inline]
    fn add(self, rhs: Mat3) -> Self::Output {
        Self::from_cols(self.c0 + rhs.c0, self.c1 + rhs.c1, self.c2 + rhs.c2)
    }
}

impl Sub<Mat3> for Mat3 {
    type Output = Mat3;

    #[inline]
    fn sub(self, rhs: Mat3) -> Self::Output {
        Self::from_cols(self.c0 - rhs.c0, self.c1 - rhs.c1, self.c2 - rhs.c2)
    }
}

impl Mul<Mat3> for Mat3 {
    type Output = Mat3;

    #[inline]
    fn mul(self, rhs: Mat3) -> Self::Output {
        let r0 = self.row(0);
        let r1 = self.row(1);
        let r2 = self.row(2);

        let c0 = rhs.c0;
        let c1 = rhs.c1;
        let c2 = rhs.c2;

        Self {
            c0: Vec3::new(r0.dot(c0), r1.dot(c0), r2.dot(c0)),
            c1: Vec3::new(r0.dot(c1), r1.dot(c1), r2.dot(c1)),
            c2: Vec3::new(r0.dot(c2), r1.dot(c2), r2.dot(c2)),
        }
    }
}

impl Div<Mat3> for Mat3 {
    type Output = Mat3;

    #[inline]
    fn div(self, rhs: Mat3) -> Self::Output {
        let r0 = self.row(0);
        let r1 = self.row(1);
        let r2 = self.row(2);

        let c0 = 1.0 / rhs.c0;
        let c1 = 1.0 / rhs.c1;
        let c2 = 1.0 / rhs.c2;

        Self {
            c0: Vec3::new(r0.dot(c0), r1.dot(c0), r2.dot(c0)),
            c1: Vec3::new(r0.dot(c1), r1.dot(c1), r2.dot(c1)),
            c2: Vec3::new(r0.dot(c2), r1.dot(c2), r2.dot(c2)),
        }
    }
}

impl Neg for Mat3 {
    type Output = Mat3;

    #[inline]
    fn neg(self) -> Self::Output {
        Self::from_cols(-self.c0, -self.c1, -self.c2)
    }
}

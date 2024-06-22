use super::{Euler, EulerOrder, Mat2, Mat3, Quat, Vec3, Vec4};
use std::mem;
use std::ops::{Add, Div, Index, IndexMut, Mul, Neg, Sub};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Mat4 {
    pub c0: Vec4,
    pub c1: Vec4,
    pub c2: Vec4,
    pub c3: Vec4,
}

impl Mat4 {
    #[rustfmt::skip]
    #[inline]
    pub fn new(c0r0: f32, c0r1: f32, c0r2: f32, c0r3: f32, c1r0: f32, c1r1: f32, c1r2: f32, c1r3: f32, c2r0: f32, c2r1: f32, c2r2: f32, c2r3: f32, c3r0: f32, c3r1: f32, c3r2: f32, c3r3: f32) -> Self {
        Self {
            c0: Vec4::new(c0r0, c0r1, c0r2, c0r3),
            c1: Vec4::new(c1r0, c1r1, c1r2, c1r3),
            c2: Vec4::new(c2r0, c2r1, c2r2, c2r3),
            c3: Vec4::new(c3r0, c3r1, c3r2, c3r3),
        }
    }

    #[inline]
    pub fn identity() -> Mat4 {
        Self {
            c0: Vec4::new(1.0, 0.0, 0.0, 0.0),
            c1: Vec4::new(0.0, 1.0, 0.0, 0.0),
            c2: Vec4::new(0.0, 0.0, 1.0, 0.0),
            c3: Vec4::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    #[inline]
    pub fn from_rows(row0: Vec4, row1: Vec4, row2: Vec4, row3: Vec4) -> Mat4 {
        Self {
            c0: Vec4::new(row0.x, row1.x, row2.x, row3.x),
            c1: Vec4::new(row0.y, row1.y, row2.y, row3.y),
            c2: Vec4::new(row0.z, row1.z, row2.z, row3.z),
            c3: Vec4::new(row0.w, row1.w, row2.w, row3.w),
        }
    }

    #[inline]
    pub fn from_cols(col0: Vec4, col1: Vec4, col2: Vec4, col3: Vec4) -> Mat4 {
        Self {
            c0: Vec4::new(col0.x, col0.y, col0.z, col0.w),
            c1: Vec4::new(col1.x, col1.y, col1.z, col1.w),
            c2: Vec4::new(col2.x, col2.y, col2.z, col2.w),
            c3: Vec4::new(col3.x, col3.y, col3.z, col3.w),
        }
    }

    #[inline]
    pub fn translation(value: Vec3) -> Self {
        Self {
            c0: Vec4::new(1.0, 0.0, 0.0, value.x),
            c1: Vec4::new(0.0, 1.0, 0.0, value.y),
            c2: Vec4::new(0.0, 0.0, 1.0, value.z),
            c3: Vec4::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    #[inline]
    pub fn rotation(rotation: Euler) -> Self {
        rotation.into()
    }

    #[inline]
    pub fn scale(value: &Vec3) -> Self {
        Self {
            c0: Vec4::new(value.x, 0.0, 0.0, 0.0),
            c1: Vec4::new(0.0, value.y, 0.0, 0.0),
            c2: Vec4::new(0.0, 0.0, value.z, 0.0),
            c3: Vec4::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    #[inline]
    pub fn row(&self, index: usize) -> Vec4 {
        let data = self.as_ref();
        Vec4::new(
            data[index],
            data[index + 4],
            data[index + 8],
            data[index + 12],
        )
    }

    #[inline]
    pub fn invert(&mut self) -> &mut Self {
        let c0r0_cof = {
            let xy_cof = self.c2.z * self.c3.w - self.c2.w * self.c3.z;
            let xz_cof = self.c2.w * self.c3.y - self.c2.y * self.c3.w;
            let xw_cof = self.c2.y * self.c3.z - self.c2.z * self.c3.y;
            self.c1.y * xy_cof + self.c1.z * xz_cof + self.c1.w * xw_cof
        };
        let c0r1_cof = {
            let yz_cof = self.c2.w * self.c3.x - self.c2.x * self.c3.w;
            let yw_cof = self.c2.x * self.c3.z - self.c2.z * self.c3.x;
            let yx_cof = self.c2.z * self.c3.w - self.c2.w * self.c3.z;
            self.c1.z * yz_cof + self.c1.w * yw_cof + self.c1.x * yx_cof
        };
        let c0r2_cof = {
            let zw_cof = self.c2.x * self.c3.y - self.c2.y * self.c3.x;
            let zx_cof = self.c2.y * self.c3.w - self.c2.w * self.c3.y;
            let zy_cof = self.c2.w * self.c3.x - self.c2.x * self.c3.w;
            self.c1.w * zw_cof + self.c1.x * zx_cof + self.c1.y * zy_cof
        };
        let c0r3_cof = {
            let wx_cof = self.c2.y * self.c3.z - self.c2.z * self.c3.y;
            let wy_cof = self.c2.z * self.c3.x - self.c2.x * self.c3.z;
            let wz_cof = self.c2.x * self.c3.y - self.c2.y * self.c3.x;
            self.c1.x * wx_cof + self.c1.y * wy_cof + self.c1.z * wz_cof
        };

        let det = self.c0.x * c0r0_cof
            + self.c0.y * c0r1_cof
            + self.c0.z * c0r2_cof
            + self.c0.w * c0r3_cof;
        if det == 0.0 {
            return self;
        }

        let c1r0_cof = {
            let xy_cof = self.c3.z * self.c0.w - self.c3.w * self.c0.z;
            let xz_cof = self.c3.w * self.c0.y - self.c3.y * self.c0.w;
            let xw_cof = self.c3.y * self.c0.z - self.c3.z * self.c0.y;
            self.c2.y * xy_cof + self.c2.z * xz_cof + self.c2.w * xw_cof
        };
        let c1r1_cof = {
            let yz_cof = self.c3.w * self.c0.x - self.c3.x * self.c0.w;
            let yw_cof = self.c3.x * self.c0.z - self.c3.z * self.c0.x;
            let yx_cof = self.c3.z * self.c0.w - self.c3.w * self.c0.z;
            self.c2.z * yz_cof + self.c2.w * yw_cof + self.c2.x * yx_cof
        };
        let c1r2_cof = {
            let zw_cof = self.c3.x * self.c0.y - self.c3.y * self.c0.x;
            let zx_cof = self.c3.y * self.c0.w - self.c3.w * self.c0.y;
            let zy_cof = self.c3.w * self.c0.x - self.c3.x * self.c0.w;
            self.c2.w * zw_cof + self.c2.x * zx_cof + self.c2.y * zy_cof
        };
        let c1r3_cof = {
            let wx_cof = self.c3.y * self.c0.z - self.c0.z * self.c3.y;
            let wy_cof = self.c3.z * self.c0.x - self.c0.x * self.c3.z;
            let wz_cof = self.c3.x * self.c0.y - self.c0.y * self.c3.x;
            self.c2.x * wx_cof + self.c2.y * wy_cof + self.c2.z * wz_cof
        };
        let c2r0_cof = {
            let xy_cof = self.c0.z * self.c1.w - self.c0.w * self.c1.z;
            let xz_cof = self.c0.w * self.c1.y - self.c0.y * self.c1.w;
            let xw_cof = self.c0.y * self.c1.z - self.c0.z * self.c1.y;
            self.c3.y * xy_cof + self.c3.z * xz_cof + self.c3.w * xw_cof
        };
        let c2r1_cof = {
            let yz_cof = self.c0.w * self.c1.x - self.c0.x * self.c1.w;
            let yw_cof = self.c0.x * self.c1.z - self.c0.z * self.c1.x;
            let yx_cof = self.c0.z * self.c1.w - self.c0.w * self.c1.z;
            self.c3.z * yz_cof + self.c3.w * yw_cof + self.c3.x * yx_cof
        };
        let c2r2_cof = {
            let zw_cof = self.c0.x * self.c1.y - self.c0.y * self.c1.x;
            let zx_cof = self.c0.y * self.c1.w - self.c0.w * self.c1.y;
            let zy_cof = self.c0.w * self.c1.x - self.c0.x * self.c1.w;
            self.c3.w * zw_cof + self.c3.x * zx_cof + self.c3.y * zy_cof
        };
        let c2r3_cof = {
            let wx_cof = self.c0.y * self.c1.z - self.c0.z * self.c1.y;
            let wy_cof = self.c0.z * self.c1.x - self.c0.x * self.c1.z;
            let wz_cof = self.c0.x * self.c1.y - self.c0.y * self.c1.x;
            self.c3.x * wx_cof + self.c3.y * wy_cof + self.c3.z * wz_cof
        };
        let c3r0_cof = {
            let xy_cof = self.c1.z * self.c2.w - self.c1.w * self.c2.z;
            let xz_cof = self.c1.w * self.c2.y - self.c1.y * self.c2.w;
            let xw_cof = self.c1.y * self.c2.z - self.c1.z * self.c2.y;
            self.c0.y * xy_cof + self.c0.z * xz_cof + self.c0.w * xw_cof
        };
        let c3r1_cof = {
            let yz_cof = self.c1.w * self.c2.x - self.c1.x * self.c2.w;
            let yw_cof = self.c1.x * self.c2.z - self.c1.z * self.c2.x;
            let yx_cof = self.c1.z * self.c2.w - self.c1.w * self.c2.z;
            self.c0.z * yz_cof + self.c0.w * yw_cof + self.c0.x * yx_cof
        };
        let c3r2_cof = {
            let zw_cof = self.c1.x * self.c2.y - self.c1.y * self.c2.x;
            let zx_cof = self.c1.y * self.c2.w - self.c1.w * self.c2.y;
            let zy_cof = self.c1.w * self.c2.x - self.c1.x * self.c2.w;
            self.c0.w * zw_cof + self.c0.x * zx_cof + self.c0.y * zy_cof
        };
        let c3r3_cof = {
            let wx_cof = self.c1.y * self.c2.z - self.c1.z * self.c2.y;
            let wy_cof = self.c1.z * self.c2.x - self.c1.x * self.c2.z;
            let wz_cof = self.c1.x * self.c2.y - self.c1.y * self.c2.x;
            self.c0.x * wx_cof + self.c0.y * wy_cof + self.c0.z * wz_cof
        };

        let det_recip = 1.0 / det;
        self.c0.x = c0r0_cof * det_recip;
        self.c0.y = c1r0_cof * det_recip;
        self.c0.z = c2r0_cof * det_recip;
        self.c0.w = c3r0_cof * det_recip;
        self.c1.x = c0r1_cof * det_recip;
        self.c1.y = c1r1_cof * det_recip;
        self.c1.z = c2r1_cof * det_recip;
        self.c1.w = c3r1_cof * det_recip;
        self.c2.x = c0r2_cof * det_recip;
        self.c2.y = c1r2_cof * det_recip;
        self.c2.z = c2r2_cof * det_recip;
        self.c2.w = c3r2_cof * det_recip;
        self.c3.x = c0r3_cof * det_recip;
        self.c3.y = c1r3_cof * det_recip;
        self.c3.z = c2r3_cof * det_recip;
        self.c3.z = c3r3_cof * det_recip;
        self
    }

    #[inline]
    pub fn transpose(&mut self) -> &mut Self {
        (self.c0.y, self.c1.x) = (self.c1.x, self.c0.y);
        (self.c0.z, self.c2.x) = (self.c2.x, self.c0.z);
        (self.c0.w, self.c3.x) = (self.c3.x, self.c0.w);
        (self.c1.z, self.c2.y) = (self.c2.y, self.c1.z);
        (self.c1.w, self.c3.y) = (self.c3.y, self.c1.w);
        (self.c2.w, self.c3.z) = (self.c3.z, self.c2.w);
        self
    }

    #[inline]
    pub fn determinant(&self) -> f32 {
        let c0r0_cof = {
            let xy_cof = self.c2.z * self.c3.w - self.c2.w * self.c3.z;
            let xz_cof = self.c2.w * self.c3.y - self.c2.y * self.c3.w;
            let xw_cof = self.c2.y * self.c3.z - self.c2.z * self.c3.y;
            self.c1.y * xy_cof + self.c1.z * xz_cof + self.c1.w * xw_cof
        };
        let c0r1_cof = {
            let yz_cof = self.c2.w * self.c3.x - self.c2.x * self.c3.w;
            let yw_cof = self.c2.x * self.c3.z - self.c2.z * self.c3.x;
            let yx_cof = self.c2.z * self.c3.w - self.c2.w * self.c3.z;
            self.c1.z * yz_cof + self.c1.w * yw_cof + self.c1.x * yx_cof
        };
        let c0r2_cof = {
            let zw_cof = self.c2.x * self.c3.y - self.c2.y * self.c3.x;
            let zx_cof = self.c2.y * self.c3.w - self.c2.w * self.c3.y;
            let zy_cof = self.c2.w * self.c3.x - self.c2.x * self.c3.w;
            self.c1.w * zw_cof + self.c1.x * zx_cof + self.c1.y * zy_cof
        };
        let c0r3_cof = {
            let wx_cof = self.c2.y * self.c3.z - self.c2.z * self.c3.y;
            let wy_cof = self.c2.z * self.c3.x - self.c2.x * self.c3.z;
            let wz_cof = self.c2.x * self.c3.y - self.c2.y * self.c3.x;
            self.c1.x * wx_cof + self.c1.y * wy_cof + self.c1.z * wz_cof
        };

        self.c0.x * c0r0_cof + self.c0.y * c0r1_cof + self.c0.z * c0r2_cof + self.c0.w * c0r3_cof
    }

    #[inline]
    pub fn decompose(&mut self, vec3: &Vec3) -> (Vec3, Quat, Vec3) {
        (
            Vec3::new(1.0, 0.0, 1.0),
            Quat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                s: 1.0,
            },
            Vec3::new(1.0, 0.0, 1.0),
        )
    }
}

impl Default for Mat4 {
    #[inline]
    fn default() -> Self {
        Self {
            c0: Vec4::default(),
            c1: Vec4::default(),
            c2: Vec4::default(),
            c3: Vec4::default(),
        }
    }
}

impl From<Euler> for Mat4 {
    #[inline]
    fn from(value: Euler) -> Self {
        let cos_x = value.x.cos();
        let sin_x = value.x.sin();
        let cos_y = value.y.cos();
        let sin_y = value.y.sin();
        let cos_z = value.z.cos();
        let sin_z = value.z.sin();

        match value.order {
            EulerOrder::ZYX => Self {
                c0: Vec4::new(cos_z * cos_y, sin_z * cos_y, -sin_y, 0.0),
                c1: Vec4::new(
                    cos_z * sin_y * sin_x - sin_z * cos_x,
                    sin_z * sin_y * sin_x + cos_z * cos_x,
                    cos_y * sin_x,
                    0.0,
                ),
                c2: Vec4::new(
                    cos_z * sin_y * cos_x + sin_z * sin_x,
                    sin_z * sin_y * cos_x - cos_z * sin_x,
                    cos_y * cos_x,
                    0.0,
                ),
                c3: Vec4::new(0.0, 0.0, 0.0, 1.0),
            },
            _ => unreachable!(),
        }
    }
}

impl From<Quat> for Mat4 {
    #[inline]
    fn from(value: Quat) -> Self {
        Self::default()
    }
}

impl From<Mat2> for Mat4 {
    #[inline]
    fn from(value: Mat2) -> Self {
        Self {
            c0: Vec4::from(value.c0),
            c1: Vec4::from(value.c1),
            c2: Vec4::new(0.0, 0.0, 1.0, 0.0),
            c3: Vec4::new(0.0, 0.0, 0.0, 1.0),
        }
    }
}

impl From<Mat3> for Mat4 {
    #[inline]
    fn from(value: Mat3) -> Self {
        Self {
            c0: Vec4::from(value.c0),
            c1: Vec4::from(value.c1),
            c2: Vec4::from(value.c2),
            c3: Vec4::new(0.0, 0.0, 0.0, 1.0),
        }
    }
}

impl AsRef<[f32; 16]> for Mat4 {
    #[inline]
    fn as_ref(&self) -> &[f32; 16] {
        unsafe { mem::transmute(self) }
    }
}

impl AsMut<[f32; 16]> for Mat4 {
    #[inline]
    fn as_mut(&mut self) -> &mut [f32; 16] {
        unsafe { mem::transmute(self) }
    }
}

impl Index<usize> for Mat4 {
    type Output = f32;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl IndexMut<usize> for Mat4 {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.as_mut().index_mut(index)
    }
}

impl Add<Mat4> for Mat4 {
    type Output = Mat4;

    #[inline]
    fn add(self, rhs: Mat4) -> Self::Output {
        Self::from_cols(
            self.c0 + rhs.c0,
            self.c1 + rhs.c1,
            self.c2 + rhs.c2,
            self.c3 + rhs.c3,
        )
    }
}

impl Sub<Mat4> for Mat4 {
    type Output = Mat4;

    #[inline]
    fn sub(self, rhs: Mat4) -> Self::Output {
        Self::from_cols(
            self.c0 - rhs.c0,
            self.c1 - rhs.c1,
            self.c2 - rhs.c2,
            self.c3 - rhs.c3,
        )
    }
}

impl Mul<Mat4> for Mat4 {
    type Output = Mat4;

    #[inline]
    fn mul(self, rhs: Mat4) -> Self::Output {
        let r0 = self.row(0);
        let r1 = self.row(1);
        let r2 = self.row(2);
        let r3 = self.row(3);

        let c0 = rhs.c0;
        let c1 = rhs.c1;
        let c2 = rhs.c2;
        let c3 = rhs.c3;

        Self {
            c0: Vec4::new(r0.dot(c0), r1.dot(c0), r2.dot(c0), r3.dot(c0)),
            c1: Vec4::new(r0.dot(c1), r1.dot(c1), r2.dot(c1), r3.dot(c1)),
            c2: Vec4::new(r0.dot(c2), r1.dot(c2), r2.dot(c2), r3.dot(c2)),
            c3: Vec4::new(r0.dot(c3), r1.dot(c3), r2.dot(c3), r3.dot(c3)),
        }
    }
}

impl Div<Mat4> for Mat4 {
    type Output = Mat4;

    #[inline]
    fn div(self, rhs: Mat4) -> Self::Output {
        let r0 = self.row(0);
        let r1 = self.row(1);
        let r2 = self.row(2);
        let r3 = self.row(3);

        let c0 = 1.0 / rhs.c0;
        let c1 = 1.0 / rhs.c1;
        let c2 = 1.0 / rhs.c2;
        let c3 = 1.0 / rhs.c3;

        Self {
            c0: Vec4::new(r0.dot(c0), r1.dot(c0), r2.dot(c0), r3.dot(c0)),
            c1: Vec4::new(r0.dot(c1), r1.dot(c1), r2.dot(c1), r3.dot(c1)),
            c2: Vec4::new(r0.dot(c2), r1.dot(c2), r2.dot(c2), r3.dot(c2)),
            c3: Vec4::new(r0.dot(c3), r1.dot(c3), r2.dot(c3), r3.dot(c3)),
        }
    }
}

impl Neg for Mat4 {
    type Output = Mat4;

    #[inline]
    fn neg(self) -> Self::Output {
        Self::from_cols(-self.c0, -self.c1, -self.c2, -self.c3)
    }
}

use super::{Euler, EulerOrder, Mat, Quat, Vec3, Vec4};

pub type Mat4 = Mat<f32, 4, 4>;

impl Mat4 {
    #[rustfmt::skip]
    #[inline]
    pub fn new(c0r0: f32, c0r1: f32, c0r2: f32, c0r3: f32, c1r0: f32, c1r1: f32, c1r2: f32, c1r3: f32, c2r0: f32, c2r1: f32, c2r2: f32, c2r3: f32, c3r0: f32, c3r1: f32, c3r2: f32, c3r3: f32) -> Self {
        Self::from([
            [c0r0, c0r1, c0r2, c0r3],
            [c1r0, c1r1, c1r2, c1r3],
            [c2r0, c2r1, c2r2, c2r3],
            [c3r0, c3r1, c3r2, c3r3],
        ])
    }

    #[inline]
    pub fn translate(value: Vec3) -> Self {
        Self::from([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [value.x, value.y, value.z, 1.0],
        ])
    }

    // rotation to 3 shear matrices Paeth Decomposition of Rotations
    #[inline]
    pub fn rotate(rotation: Euler) -> Self {
        Self::from(rotation)
    }

    #[inline]
    pub fn scale(value: Vec3) -> Self {
        Self::from([
            [value.x, 0.0, 0.0, 0.0],
            [0.0, value.y, 0.0, 0.0],
            [0.0, 0.0, value.z, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    fn look_at(eye: Vec3, dir: Vec3, up: Vec3) -> Self {
        let z = dir.normalize();
        let x = up.cross(z).normalize();
        let y = z.cross(x);

        Self::from([
            [x.x, y.x, z.x, 0.0],
            [x.y, y.y, z.y, 0.0],
            [x.z, y.z, z.z, 0.0],
            [-eye.dot(x), -eye.dot(y), -eye.dot(z), 1.0],
        ])
    }

    pub fn look_at_lh(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        Self::look_at(eye, target - eye, up)
    }

    pub fn look_at_rh(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        Self::look_at(eye, eye - target, up)
    }

    /**
     * LH; Y downward; Z [0, 1]
     * orthographic matrix
     * |     2/(r-l)       0          0     -(r+l)/(r-l)  |
     * |        0       2/(b-t)       0     -(t+b)/(b-t)  |
     * |        0          0       1/(f-n)    -n/(f-n)    |
     * |        0          0          0          1        |
     *
     * perspective matrix
     * |     n     0     0     0     |
     * |     0     n     0     0     |
     * |     0     0    n+f   -nf    |
     * |     0     0     1     0     |
     *
     * perspective projection matrix
     * |     2n/(r-l)      0     -(r+l)/(r-l)    0        |
     * |        0       2n/(b-t) -(t+b)/(b-t)    0        |
     * |        0          0        f/(f-n)   -nf/(f-n)   |
     * |        0          0          1          0        |
     *
     * r = -l, t = -b; θ = fov_y/2, h = t-b = 2n*tan(θ), w = r-l = h * aspect
     * | 1/(tan(θ)*asp)    0          0          0        |
     * |        0       -1/tan(θ)     0          0        |
     * |        0          0       f/(f-n)     -nf/(f-n)  |
     * |        0          0          1          0        |
     *
     * https://developer.nvidia.com/content/depth-precision-visualized
     * reversed-Z orthographic matrix
     * |     2/(r-l)       0          0     -(r+l)/(r-l)  |
     * |        0       2/(b-t)       0     -(t+b)/(b-t)  |
     * |        0          0      -1/(f-n)     f/(f-n)    |
     * |        0          0          0          1        |
     *
     * reversed-Z perspective projection matrix
     * | 1/(tan(θ)*asp)    0          0          0        |
     * |        0       -1/tan(θ)     0          0        |
     * |        0          0      -n/(f-n)     -nf/(f-n)  |
     * |        0          0          1          0        |
     *
     * Infinite perspective
     *  lim (f -> ∞) n / (f - n) = 0
     *  lim (f -> ∞) nf / (f - n) = n
     */
    pub fn orthographic_lh(
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near_z: f32,
        far_z: f32,
    ) -> Self {
        let recip_rl = 1.0 / (right - left);
        let recip_bt = 1.0 / (bottom - top);
        let recip_fn = 1.0 / (far_z - near_z);

        Self::from([
            [2.0 * recip_rl, 0.0, 0.0, 0.0],
            [0.0, 2.0 * recip_bt, 0.0, 0.0],
            [0.0, 0.0, recip_fn, 0.0],
            [
                -(right + left) * recip_rl,
                -(top + bottom) * recip_bt,
                -near_z * recip_fn,
                1.0,
            ],
        ])
    }

    pub fn perspective_lh(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let tan_theta_recip = 1.0 / (fov_y / 2.0).tan();
        Self::from([
            [tan_theta_recip / aspect, 0.0, 0.0, 0.0],
            [0.0, -tan_theta_recip, 0.0, 0.0],
            [0.0, 0.0, far / (far - near), 1.0],
            [0.0, 0.0, near * far / (near - far), 0.0],
        ])
    }

    pub fn perspective_reversed_z_lh(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let tan_theta_recip = 1.0 / (fov_y / 2.0).tan();
        Self::from([
            [tan_theta_recip / aspect, 0.0, 0.0, 0.0],
            [0.0, -tan_theta_recip, 0.0, 0.0],
            [0.0, 0.0, near / (near - far), 1.0],
            [0.0, 0.0, near * far / (near - far), 0.0],
        ])
    }

    pub fn perspective_reversed_z_infinite_lh(fov_y: f32, aspect: f32, near: f32) -> Self {
        let tan_theta_recip = 1.0 / (fov_y / 2.0).tan();
        Self::from([
            [tan_theta_recip / aspect, 0.0, 0.0, 0.0],
            [0.0, -tan_theta_recip, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            [0.0, 0.0, -near, 0.0],
        ])
    }

    /**
     * RH; Y downward; Z range [0, 1]
     * orthographic matrix
     * |     2/(r-l)       0          0     -(r+l)/(r-l)  |
     * |        0       2/(b-t)       0     -(t+b)/(b-t)  |
     * |        0          0       -1/(f-n)   -n/(f-n)    |
     * |        0          0          0          1        |
     *
     * perspective matrix
     * |     n     0     0     0     |
     * |     0     n     0     0     |
     * |     0     0    n+f    nf    |
     * |     0     0    -1     0     |
     *
     * perspective projection matrix
     * |     2n/(r-l)      0      (r+l)/(r-l)    0        |
     * |        0       2n/(b-t)  (t+b)/(b-t)    0        |
     * |        0          0       -f/(f-n)   -nf/(f-n)   |
     * |        0          0         -1          0        |
     *
     * r = -l, t = -b; θ = fov_y/2, h = t-b = 2n*tan(θ), w = r-l = h * aspect
     * |  1/(tan(θ)*asp)   0          0          0        |
     * |        0      -1/tan(θ)      0          0        |
     * |        0          0       -f/(f-n)   -nf/(f-n)   |
     * |        0          0         -1          0        |
     *
     * https://developer.nvidia.com/content/depth-precision-visualized
     * reversed-Z orthographic matrix
     * |     2/(r-l)       0          0     -(r+l)/(r-l)  |
     * |        0       2/(b-t)       0     -(t+b)/(b-t)  |
     * |        0          0        1/(f-n)    f/(f-n)    |
     * |        0          0          0          1        |
     *
     * reversed-Z perspective projection matrix
     * |  1/(tan(θ)*asp)   0          0          0        |
     * |        0      -1/tan(θ)      0          0        |
     * |        0          0        n/(f-n)    nf/(f-n)   |
     * |        0          0         -1          0        |
     *
     * Infinite perspective
     *  lim (f -> ∞)  n / (f - n) = 0
     *  lim (f -> ∞) nf / (f - n) = n
     */
    pub fn orthographic_rh(
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near_z: f32,
        far_z: f32,
    ) -> Self {
        let recip_rl = 1.0 / (right - left);
        let recip_bt = 1.0 / (bottom - top);
        let recip_fn = 1.0 / (far_z - near_z);

        Self::from([
            [2.0 * recip_rl, 0.0, 0.0, 0.0],
            [0.0, 2.0 * recip_bt, 0.0, 0.0],
            [0.0, 0.0, -recip_fn, 0.0],
            [
                -(right + left) * recip_rl,
                -(top + bottom) * recip_bt,
                -near_z * recip_fn,
                1.0,
            ],
        ])
    }

    pub fn perspective_rh(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let tan_theta_recip = 1.0 / (fov_y / 2.0).tan();
        Self::from([
            [tan_theta_recip / aspect, 0.0, 0.0, 0.0],
            [0.0, -tan_theta_recip, 0.0, 0.0],
            [0.0, 0.0, far / (near - far), -1.0],
            [0.0, 0.0, near * far / (near - far), 0.0],
        ])
    }

    pub fn perspective_reversed_z_rh(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let tan_theta_recip = 1.0 / (fov_y / 2.0).tan();
        Self::from([
            [tan_theta_recip / aspect, 0.0, 0.0, 0.0],
            [0.0, -tan_theta_recip, 0.0, 0.0],
            [0.0, 0.0, near / (far - near), -1.0],
            [0.0, 0.0, near * far / (far - near), 0.0],
        ])
    }

    pub fn perspective_reversed_z_infinite_rh(fov_y: f32, aspect: f32, near: f32) -> Self {
        let tan_theta_recip = 1.0 / (fov_y / 2.0).tan();
        Self::from([
            [tan_theta_recip / aspect, 0.0, 0.0, 0.0],
            [0.0, -tan_theta_recip, 0.0, 0.0],
            [0.0, 0.0, 0.0, -1.0],
            [0.0, 0.0, near, 0.0],
        ])
    }

    #[inline]
    pub fn compose(location: Vec3, rotation: Euler, scale: Vec3) -> Self {
        let mut mat = Self::rotate(rotation) * Self::scale(scale);
        mat[3] = [location.x, location.y, location.z, 1.0];
        mat
    }

    #[inline]
    pub fn decompose(mat4: Self) -> (Vec3, Euler, Vec3) {
        (
            Vec3::new(1.0, 0.0, 1.0),
            // Quat {
            //     x: 0.0,
            //     y: 0.0,
            //     z: 0.0,
            //     s: 1.0,
            // },
            Euler {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                order: EulerOrder::ZYX,
            },
            Vec3::new(1.0, 0.0, 1.0),
        )
    }

    pub fn invert_svd(&self) -> Self {
        Self::default()
    }

    #[inline]
    pub fn invert(&self) -> Self {
        let c0 = Vec4::from(self.col(0));
        let c1 = Vec4::from(self.col(1));
        let c2 = Vec4::from(self.col(2));
        let c3 = Vec4::from(self.col(3));

        let c0r0_cof = {
            let xy_cof = c2.z * c3.w - c2.w * c3.z;
            let xz_cof = c2.w * c3.y - c2.y * c3.w;
            let xw_cof = c2.y * c3.z - c2.z * c3.y;
            c1.y * xy_cof + c1.z * xz_cof + c1.w * xw_cof
        };
        let c0r1_cof = {
            let yz_cof = c2.w * c3.x - c2.x * c3.w;
            let yw_cof = c2.x * c3.z - c2.z * c3.x;
            let yx_cof = c2.z * c3.w - c2.w * c3.z;
            c1.z * yz_cof + c1.w * yw_cof + c1.x * yx_cof
        };
        let c0r2_cof = {
            let zw_cof = c2.x * c3.y - c2.y * c3.x;
            let zx_cof = c2.y * c3.w - c2.w * c3.y;
            let zy_cof = c2.w * c3.x - c2.x * c3.w;
            c1.w * zw_cof + c1.x * zx_cof + c1.y * zy_cof
        };
        let c0r3_cof = {
            let wx_cof = c2.y * c3.z - c2.z * c3.y;
            let wy_cof = c2.z * c3.x - c2.x * c3.z;
            let wz_cof = c2.x * c3.y - c2.y * c3.x;
            c1.x * wx_cof + c1.y * wy_cof + c1.z * wz_cof
        };

        let det = c0.x * c0r0_cof + c0.y * c0r1_cof + c0.z * c0r2_cof + c0.w * c0r3_cof;
        if det == 0.0 {
            return *self;
        }

        let c1r0_cof = {
            let xy_cof = c3.z * c0.w - c3.w * c0.z;
            let xz_cof = c3.w * c0.y - c3.y * c0.w;
            let xw_cof = c3.y * c0.z - c3.z * c0.y;
            c2.y * xy_cof + c2.z * xz_cof + c2.w * xw_cof
        };
        let c1r1_cof = {
            let yz_cof = c3.w * c0.x - c3.x * c0.w;
            let yw_cof = c3.x * c0.z - c3.z * c0.x;
            let yx_cof = c3.z * c0.w - c3.w * c0.z;
            c2.z * yz_cof + c2.w * yw_cof + c2.x * yx_cof
        };
        let c1r2_cof = {
            let zw_cof = c3.x * c0.y - c3.y * c0.x;
            let zx_cof = c3.y * c0.w - c3.w * c0.y;
            let zy_cof = c3.w * c0.x - c3.x * c0.w;
            c2.w * zw_cof + c2.x * zx_cof + c2.y * zy_cof
        };
        let c1r3_cof = {
            let wx_cof = c3.y * c0.z - c0.z * c3.y;
            let wy_cof = c3.z * c0.x - c0.x * c3.z;
            let wz_cof = c3.x * c0.y - c0.y * c3.x;
            c2.x * wx_cof + c2.y * wy_cof + c2.z * wz_cof
        };
        let c2r0_cof = {
            let xy_cof = c0.z * c1.w - c0.w * c1.z;
            let xz_cof = c0.w * c1.y - c0.y * c1.w;
            let xw_cof = c0.y * c1.z - c0.z * c1.y;
            c3.y * xy_cof + c3.z * xz_cof + c3.w * xw_cof
        };
        let c2r1_cof = {
            let yz_cof = c0.w * c1.x - c0.x * c1.w;
            let yw_cof = c0.x * c1.z - c0.z * c1.x;
            let yx_cof = c0.z * c1.w - c0.w * c1.z;
            c3.z * yz_cof + c3.w * yw_cof + c3.x * yx_cof
        };
        let c2r2_cof = {
            let zw_cof = c0.x * c1.y - c0.y * c1.x;
            let zx_cof = c0.y * c1.w - c0.w * c1.y;
            let zy_cof = c0.w * c1.x - c0.x * c1.w;
            c3.w * zw_cof + c3.x * zx_cof + c3.y * zy_cof
        };
        let c2r3_cof = {
            let wx_cof = c0.y * c1.z - c0.z * c1.y;
            let wy_cof = c0.z * c1.x - c0.x * c1.z;
            let wz_cof = c0.x * c1.y - c0.y * c1.x;
            c3.x * wx_cof + c3.y * wy_cof + c3.z * wz_cof
        };
        let c3r0_cof = {
            let xy_cof = c1.z * c2.w - c1.w * c2.z;
            let xz_cof = c1.w * c2.y - c1.y * c2.w;
            let xw_cof = c1.y * c2.z - c1.z * c2.y;
            c0.y * xy_cof + c0.z * xz_cof + c0.w * xw_cof
        };
        let c3r1_cof = {
            let yz_cof = c1.w * c2.x - c1.x * c2.w;
            let yw_cof = c1.x * c2.z - c1.z * c2.x;
            let yx_cof = c1.z * c2.w - c1.w * c2.z;
            c0.z * yz_cof + c0.w * yw_cof + c0.x * yx_cof
        };
        let c3r2_cof = {
            let zw_cof = c1.x * c2.y - c1.y * c2.x;
            let zx_cof = c1.y * c2.w - c1.w * c2.y;
            let zy_cof = c1.w * c2.x - c1.x * c2.w;
            c0.w * zw_cof + c0.x * zx_cof + c0.y * zy_cof
        };
        let c3r3_cof = {
            let wx_cof = c1.y * c2.z - c1.z * c2.y;
            let wy_cof = c1.z * c2.x - c1.x * c2.z;
            let wz_cof = c1.x * c2.y - c1.y * c2.x;
            c0.x * wx_cof + c0.y * wy_cof + c0.z * wz_cof
        };

        let det_recip = 1.0 / det;
        Self::from([
            [
                c0r0_cof * det_recip,
                c1r0_cof * det_recip,
                c2r0_cof * det_recip,
                c3r0_cof * det_recip,
            ],
            [
                c0r1_cof * det_recip,
                c1r1_cof * det_recip,
                c2r1_cof * det_recip,
                c3r1_cof * det_recip,
            ],
            [
                c0r2_cof * det_recip,
                c1r2_cof * det_recip,
                c2r2_cof * det_recip,
                c3r2_cof * det_recip,
            ],
            [
                c0r3_cof * det_recip,
                c1r3_cof * det_recip,
                c2r3_cof * det_recip,
                c3r3_cof * det_recip,
            ],
        ])
    }

    #[inline]
    pub fn transpose(&self) -> Self {
        Mat4::from_rows([self.col(0), self.col(1), self.col(2), self.col(3)])
    }

    #[inline]
    pub fn determinant(&self) -> f32 {
        let c0 = Vec4::from(self.col(0));
        let c1 = Vec4::from(self.col(1));
        let c2 = Vec4::from(self.col(2));
        let c3 = Vec4::from(self.col(3));

        let c0r0_cof = {
            let xy_cof = c2.z * c3.w - c2.w * c3.z;
            let xz_cof = c2.w * c3.y - c2.y * c3.w;
            let xw_cof = c2.y * c3.z - c2.z * c3.y;
            c1.y * xy_cof + c1.z * xz_cof + c1.w * xw_cof
        };
        let c0r1_cof = {
            let yz_cof = c2.w * c3.x - c2.x * c3.w;
            let yw_cof = c2.x * c3.z - c2.z * c3.x;
            let yx_cof = c2.z * c3.w - c2.w * c3.z;
            c1.z * yz_cof + c1.w * yw_cof + c1.x * yx_cof
        };
        let c0r2_cof = {
            let zw_cof = c2.x * c3.y - c2.y * c3.x;
            let zx_cof = c2.y * c3.w - c2.w * c3.y;
            let zy_cof = c2.w * c3.x - c2.x * c3.w;
            c1.w * zw_cof + c1.x * zx_cof + c1.y * zy_cof
        };
        let c0r3_cof = {
            let wx_cof = c2.y * c3.z - c2.z * c3.y;
            let wy_cof = c2.z * c3.x - c2.x * c3.z;
            let wz_cof = c2.x * c3.y - c2.y * c3.x;
            c1.x * wx_cof + c1.y * wy_cof + c1.z * wz_cof
        };

        c0.x * c0r0_cof + c0.y * c0r1_cof + c0.z * c0r2_cof + c0.w * c0r3_cof
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
            EulerOrder::ZYX => Self::from([
                [cos_z * cos_y, sin_z * cos_y, -sin_y, 0.0],
                [
                    cos_z * sin_y * sin_x - sin_z * cos_x,
                    sin_z * sin_y * sin_x + cos_z * cos_x,
                    cos_y * sin_x,
                    0.0,
                ],
                [
                    cos_z * sin_y * cos_x + sin_z * sin_x,
                    sin_z * sin_y * cos_x - cos_z * sin_x,
                    cos_y * cos_x,
                    0.0,
                ],
                [0.0, 0.0, 0.0, 1.0],
            ]),
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
//
// impl From<Mat2> for Mat4 {
//     #[inline]
//     fn from(value: Mat2) -> Self {
//         Self {
//             c0: Vec4::from(value.c0),
//             c1: Vec4::from(value.c1),
//             c2: Vec4::new(0.0, 0.0, 1.0, 0.0),
//             c3: Vec4::new(0.0, 0.0, 0.0, 1.0),
//         }
//     }
// }
//
// impl From<Mat3> for Mat4 {
//     #[inline]
//     fn from(value: Mat3) -> Self {
//         Self {
//             c0: Vec4::from(value.c0),
//             c1: Vec4::from(value.c1),
//             c2: Vec4::from(value.c2),
//             c3: Vec4::new(0.0, 0.0, 0.0, 1.0),
//         }
//     }
// }
//
// impl Mul<Mat4> for Mat4 {
//     type Output = Mat4;
//
//     #[inline]
//     fn mul(self, rhs: Mat4) -> Self::Output {
//         let r0 = self.row(0);
//         let r1 = self.row(1);
//         let r2 = self.row(2);
//         let r3 = self.row(3);
//
//         let c0 = rhs.c0;
//         let c1 = rhs.c1;
//         let c2 = rhs.c2;
//         let c3 = rhs.c3;
//
//         Self {
//             c0: Vec4::new(r0.dot(c0), r1.dot(c0), r2.dot(c0), r3.dot(c0)),
//             c1: Vec4::new(r0.dot(c1), r1.dot(c1), r2.dot(c1), r3.dot(c1)),
//             c2: Vec4::new(r0.dot(c2), r1.dot(c2), r2.dot(c2), r3.dot(c2)),
//             c3: Vec4::new(r0.dot(c3), r1.dot(c3), r2.dot(c3), r3.dot(c3)),
//         }
//     }
// }
//
// impl Div<Mat4> for Mat4 {
//     type Output = Mat4;
//
//     #[inline]
//     fn div(self, rhs: Mat4) -> Self::Output {
//         let r0 = self.row(0);
//         let r1 = self.row(1);
//         let r2 = self.row(2);
//         let r3 = self.row(3);
//
//         let c0 = 1.0 / rhs.c0;
//         let c1 = 1.0 / rhs.c1;
//         let c2 = 1.0 / rhs.c2;
//         let c3 = 1.0 / rhs.c3;
//
//         Self {
//             c0: Vec4::new(r0.dot(c0), r1.dot(c0), r2.dot(c0), r3.dot(c0)),
//             c1: Vec4::new(r0.dot(c1), r1.dot(c1), r2.dot(c1), r3.dot(c1)),
//             c2: Vec4::new(r0.dot(c2), r1.dot(c2), r2.dot(c2), r3.dot(c2)),
//             c3: Vec4::new(r0.dot(c3), r1.dot(c3), r2.dot(c3), r3.dot(c3)),
//         }
//     }
// }

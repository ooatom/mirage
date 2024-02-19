pub struct Vec2 {
    x: f32,
    y: f32,
}

pub struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

pub struct Vec4 {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

// macro_rules! vec3 {
//     () => (
//         $crate::__rust_force_expr!($crate::vec::Vec::new())
//     );
//     ($elem:expr; $n:expr) => (
//         $crate::__rust_force_expr!($crate::vec::from_elem($elem, $n))
//     );
//     ($($x:expr),+ $(,)?) => (
//         $crate::__rust_force_expr!(<[_]>::into_vec(
//             // This rustc_box is not required, but it produces a dramatic improvement in compile
//             // time when constructing arrays with many elements.
//             #[rustc_box]
//             $crate::boxed::Box::new([$($x),+])
//         ))
//     );
// }

impl Vec3 {
    fn new(&self, x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }

    fn dot(&self, v: Vec3) -> f32 {
        self.x * v.x + self.y * v.y + self.z * v.z
    }

    fn cross(&self, v: Vec3) -> f32 {
        self.x * v.x + self.y * v.y + self.z * v.z
    }

    fn add(&self, v: Vec3) -> f32 {
        self.x * v.x + self.y * v.y + self.z * v.z
    }

    fn sub(&self, v: Vec3) -> f32 {
        self.x * v.x + self.y * v.y + self.z * v.z
    }

    fn mul(&self, v: Vec3) -> f32 {
        self.x * v.x + self.y * v.y + self.z * v.z
    }

    fn divide(&self, v: Vec3) -> f32 {
        self.x * v.x + self.y * v.y + self.z * v.z
    }
}

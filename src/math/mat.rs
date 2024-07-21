use num_traits::{Num, One, Signed};
use std::mem;
use std::ops::{Add, Div, Index, IndexMut, Mul, Neg, Sub};

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Mat<T, const C: usize, const R: usize> {
    m: [[T; R]; C],
}

impl<T: Default + Copy, const C: usize, const R: usize> Mat<T, C, R> {
    pub fn dimension(&self) -> (usize, usize) {
        (C, R)
    }

    pub fn col(&self, index: usize) -> [T; R] {
        self[index]
    }

    pub fn row(&self, index: usize) -> [T; C] {
        let mut data = [Default::default(); C];
        for col in 0..C {
            data[col] = self[col][index]
        }

        data
    }

    pub fn from_rows(rows: [[T; R]; C]) -> Self {
        let mut m = [[Default::default(); R]; C];

        for col in 0..C {
            for row in 0..R {
                m[col][row] = rows[row][col]
            }
        }

        Self { m }
    }
}

impl<T: Num + Default + Copy, const D: usize> Mat<T, D, D> {
    // diagonal
    // A 0
    // 0 B
    // orthogonal / orthonormal (for vector there are different)
    // all colums length equate to 1, and orthogonal to each other

    pub fn is_symmetric(&self) -> bool {
        true
    }

    pub fn eigenvalues(&self) -> Option<Vec<[T; D]>> {
        if self.is_symmetric() {
            return None;
        }
        None
    }

    pub fn singular_values(&self) -> Option<Vec<[T; D]>> {
        if self.is_symmetric() {
            return None;
        }
        None
    }

    pub fn eigenvalues_decompose(&self) -> Option<Vec<[T; D]>> {
        if self.is_symmetric() {
            return None;
        }
        None
    }

    pub fn singular_values_decompose(&self) -> Option<Vec<[T; D]>> {
        if self.is_symmetric() {
            return self.eigenvalues_decompose();
        }
        None
    }

    pub fn identity() -> Self {
        let mut out = Mat::default();
        for i in 0..D {
            out[i][i] = One::one();
        }
        out
    }

    // pub fn invert(&mut self) -> &mut Self {
    //     self
    // }
    // pub fn transpose(&mut self) -> &mut Self {
    //     self
    // }
    //
    // pub fn determinant(&self) {
    //     &mut Self::default();
    // }
}

impl<T: Default + Copy, const C: usize, const R: usize> Default for Mat<T, C, R> {
    fn default() -> Self {
        Self {
            m: [[Default::default(); R]; C],
        }
    }
}

impl<T, const C: usize, const R: usize> Index<usize> for Mat<T, C, R> {
    type Output = [T; R];

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.m[index]
    }
}

impl<T, const C: usize, const R: usize> IndexMut<usize> for Mat<T, C, R> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.m[index]
    }
}

impl<T, const C: usize, const R: usize> AsRef<[[T; R]; C]> for Mat<T, C, R> {
    #[inline]
    fn as_ref(&self) -> &[[T; R]; C] {
        unsafe { mem::transmute(self) }
    }
}

impl<T, const C: usize, const R: usize> AsMut<[[T; R]; C]> for Mat<T, C, R> {
    #[inline]
    fn as_mut(&mut self) -> &mut [[T; R]; C] {
        unsafe { mem::transmute(self) }
    }
}

// impl<T, const C: usize, const R: usize> AsRef<[T; C * R]> for Mat<T, C, R> {
//     #[inline]
//     fn as_ref(&self) -> &[T; C * R] {
//         unsafe { mem::transmute(self) }
//     }
// }

// impl<T, const C: usize, const R: usize> AsMut<[T; C * R]> for Mat<T, C, R> {
//     #[inline]
//     fn as_mut(&mut self) -> &mut [T; C * R] {
//         unsafe { mem::transmute(self) }
//     }
// }

impl<T, const C: usize, const R: usize> From<[[T; R]; C]> for Mat<T, C, R> {
    fn from(value: [[T; R]; C]) -> Self {
        Self { m: value }
    }
}

// impl<T, const C: usize, const R: usize> From<[T; C * R]> for Mat<T, C, R> {
//     fn from(value: [T; C * R]) -> Self {
//         Self {
//             m: unsafe { mem::transmute(value) },
//         }
//     }
// }

impl<T: Num + Default + Copy, const C: usize, const R: usize> Add for Mat<T, C, R> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut mat = Self::default();
        for col in 0..C {
            for row in 0..R {
                mat[col][row] = self[col][row] + rhs[col][row];
            }
        }
        mat
    }
}

impl<T: Num + Default + Copy, const C: usize, const R: usize> Sub for Mat<T, C, R> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut mat = Self::default();
        for col in 0..C {
            for row in 0..R {
                mat[col][row] = self[col][row] - rhs[col][row];
            }
        }
        mat
    }
}

impl<T: Num + Default + Copy, const C: usize, const R: usize> Mul for Mat<T, C, R> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut mat = Self::default();
        let count = C.min(R);
        for col in 0..C {
            for row in 0..R {
                for i in 0..count {
                    mat[col][row] = mat[col][row] + self[i][row] * rhs[col][i];
                }
            }
        }
        mat
    }
}

impl<T: Num + Default + Copy, const C: usize, const R: usize> Div for Mat<T, C, R> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let mut mat = Self::default();
        for col in 0..C {
            for row in 0..R {
                mat[col][row] = self[col][row] / rhs[col][row];
            }
        }
        mat
    }
}

impl<T: Signed + Default + Copy, const C: usize, const R: usize> Neg for Mat<T, C, R> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        let mut mat = Self::default();
        for col in 0..C {
            for row in 0..R {
                mat[col][row] = -self[col][row];
            }
        }
        mat
    }
}

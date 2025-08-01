use std::ptr::null_mut;

#[repr(C)]
#[derive(Clone)]
pub struct Matrix(pub *mut libc::c_void);

impl Matrix {
    pub fn null() -> Matrix {
        Matrix(null_mut())
    }
}

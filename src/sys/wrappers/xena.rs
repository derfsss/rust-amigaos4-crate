//! IXena global(s) and convenience wrappers.
//!
//! Hand-written binding for xena.library — the Xorro CPU
//! (AmigaOne X1000/X5000 secondary processor) resource arbiter.

use crate::sys::types::*;
use crate::sys::interfaces::xena::*;

// ---- IXena (XenaIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IXena: *mut XenaIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IXena: *mut XenaIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn xena_alloc_resource(unit: u32, name: CONST_STRPTR) -> CONST_STRPTR {
    ((*IXena).AllocResource)(IXena, unit, name)
}

#[inline]
pub unsafe fn xena_free_resource(unit: u32) {
    ((*IXena).FreeResource)(IXena, unit)
}

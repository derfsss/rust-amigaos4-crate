//! ISpace global(s) and convenience wrappers.
//!
//! Hand-written ReAction gadget-class binding. Like the other
//! single-`GetClass` gadgets, the only user-facing method is
//! `SPACE_GetClass`; lifecycle methods are framework-owned.

use crate::sys::types::*;
use crate::sys::interfaces::space::*;

// ---- ISpace (SpaceIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static ISpace: *mut SpaceIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut ISpace: *mut SpaceIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn space_space_get_class() -> *mut APTR {
    ((*ISpace).SPACE_GetClass)(ISpace)
}

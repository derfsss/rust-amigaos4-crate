//! IGetColor global(s) and convenience wrappers.
//!
//! Hand-written ReAction gadget-class binding. Like the other
//! single-`GetClass` gadgets, the only user-facing method is
//! `GETCOLOR_GetClass`; lifecycle methods are framework-owned.

use crate::sys::types::*;
use crate::sys::interfaces::getcolor::*;

// ---- IGetColor (GetColorIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IGetColor: *mut GetColorIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IGetColor: *mut GetColorIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn getcolor_getcolor_get_class() -> *mut APTR {
    ((*IGetColor).GETCOLOR_GetClass)(IGetColor)
}

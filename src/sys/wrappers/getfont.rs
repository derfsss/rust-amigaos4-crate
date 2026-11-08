//! IGetFont global(s) and convenience wrappers.
//!
//! Hand-written ReAction gadget-class binding. Like the other
//! single-`GetClass` gadgets, the only user-facing method is
//! `GETFONT_GetClass`; lifecycle methods are framework-owned.

use crate::sys::types::*;
use crate::sys::interfaces::getfont::*;

// ---- IGetFont (GetFontIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IGetFont: *mut GetFontIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IGetFont: *mut GetFontIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn getfont_getfont_get_class() -> *mut APTR {
    ((*IGetFont).GETFONT_GetClass)(IGetFont)
}

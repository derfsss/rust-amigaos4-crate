//! IString global(s) and convenience wrappers.
//!
//! Hand-written ReAction gadget-class binding. Like the other
//! single-`GetClass` gadgets, the only user-facing method is
//! `STRING_GetClass`; lifecycle methods are framework-owned.

use crate::sys::types::*;
use crate::sys::interfaces::string_gc::*;

// ---- IString (StringIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IString: *mut StringIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IString: *mut StringIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn string_gc_string_get_class() -> *mut APTR {
    ((*IString).STRING_GetClass)(IString)
}

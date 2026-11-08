//! ISketchBoard global(s) and convenience wrappers.
//!
//! Hand-written ReAction gadget-class binding. Like the other
//! single-`GetClass` gadgets, the only user-facing method is
//! `SKETCHBOARD_GetClass`; lifecycle methods are framework-owned.

use crate::sys::types::*;
use crate::sys::interfaces::sketchboard::*;

// ---- ISketchBoard (SketchBoardIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static ISketchBoard: *mut SketchBoardIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut ISketchBoard: *mut SketchBoardIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn sketchboard_sketchboard_get_class() -> *mut APTR {
    ((*ISketchBoard).SKETCHBOARD_GetClass)(ISketchBoard)
}

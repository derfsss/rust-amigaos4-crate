//! IGlyph global(s) and convenience wrappers.
//!
//! Hand-written ReAction gadget-class binding. Like the other
//! single-`GetClass` gadgets, the only user-facing method is
//! `GLYPH_GetClass`; lifecycle methods are framework-owned.

use crate::sys::types::*;
use crate::sys::interfaces::glyph::*;

// ---- IGlyph (GlyphIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IGlyph: *mut GlyphIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IGlyph: *mut GlyphIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn glyph_glyph_get_class() -> *mut APTR {
    ((*IGlyph).GLYPH_GetClass)(IGlyph)
}

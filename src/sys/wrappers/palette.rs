//! IPalette global(s) and convenience wrappers.
//!
//! Hand-written ReAction gadget-class binding. Like the other
//! single-`GetClass` gadgets, the only user-facing method is
//! `PALETTE_GetClass`; lifecycle methods are framework-owned.

use crate::sys::types::*;
use crate::sys::interfaces::palette::*;

// ---- IPalette (PaletteIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IPalette: *mut PaletteIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IPalette: *mut PaletteIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn palette_palette_get_class() -> *mut APTR {
    ((*IPalette).PALETTE_GetClass)(IPalette)
}

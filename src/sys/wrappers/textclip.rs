//! ITextClip global(s) and convenience wrappers.
//!
//! Hand-written binding for textclip.library — text-only clipboard
//! access. Complements the existing `iffparse`-based clipboard work
//! in the higher-level amigaos4 crate.

use crate::sys::types::*;
use crate::sys::interfaces::textclip::*;

// ---- ITextClip (TextClipIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static ITextClip: *mut TextClipIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut ITextClip: *mut TextClipIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn textclip_write_clip_vector(buffer: CONST_STRPTR, length: u32) -> i32 {
    ((*ITextClip).WriteClipVector)(ITextClip, buffer, length)
}

#[inline]
pub unsafe fn textclip_read_clip_vector(buffer: *mut STRPTR, length: *mut u32) -> i32 {
    ((*ITextClip).ReadClipVector)(ITextClip, buffer, length)
}

#[inline]
pub unsafe fn textclip_dispose_clip_vector(buffer: STRPTR) {
    ((*ITextClip).DisposeClipVector)(ITextClip, buffer)
}

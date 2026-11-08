//! IHunk global(s) and convenience wrappers.
//!
//! Hand-written binding for hunk.library — Amiga executable-format
//! loader. Reads and unloads the segmented "hunk" object format used
//! by AmigaOS executables and shared libraries.

use crate::sys::types::*;
use crate::sys::interfaces::hunk::*;

// ---- IHunk (HunkIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IHunk: *mut HunkIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IHunk: *mut HunkIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn hunk_load_hunk(tag_list: *const TagItem) -> u32 {
    ((*IHunk).LoadHunk)(IHunk, tag_list)
}

#[inline]
pub unsafe fn hunk_un_load_hunk(tag_list: *const TagItem) {
    ((*IHunk).UnLoadHunk)(IHunk, tag_list)
}

//! IDiffView global(s) and convenience wrappers.
//!
//! Hand-written binding for diffview.library — side-by-side text-
//! difference viewer. Construct DiffObject instances and query their
//! attributes (line counts, diff hunks, etc.).
//!
//! `Private1` and `Private2` slots in the SDK struct are typed as
//! `APTR` placeholders; they're skipped, as are the varargs entries.

use crate::sys::types::*;
use crate::sys::interfaces::diffview::*;

// ---- IDiffView (DiffViewIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IDiffView: *mut DiffViewIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IDiffView: *mut DiffViewIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn diffview_create_diff_object(tag_list: *mut TagItem) -> APTR {
    ((*IDiffView).CreateDiffObject)(IDiffView, tag_list)
}

#[inline]
pub unsafe fn diffview_free_diff_object(diff_object: APTR) {
    ((*IDiffView).FreeDiffObject)(IDiffView, diff_object)
}

#[inline]
pub unsafe fn diffview_get_diff_object_attr(diff_object: APTR, attr: u32, storage: APTR) -> u32 {
    ((*IDiffView).GetDiffObjectAttr)(IDiffView, diff_object, attr, storage)
}

//! IChooser global(s) and convenience wrappers.
//!
//! Hand-written ReAction chooser-class binding with helper methods
//! for managing the chooser's list of nodes.

use crate::sys::types::*;
use crate::sys::interfaces::chooser::*;

// ---- IChooser (ChooserIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IChooser: *mut ChooserIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IChooser: *mut ChooserIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn chooser_chooser_get_class() -> *mut APTR {
    ((*IChooser).CHOOSER_GetClass)(IChooser)
}

// ── Chooser node lifecycle ───────────────────────────────────

#[inline]
pub unsafe fn chooser_alloc_chooser_node_a(tag_list: *mut TagItem) -> *mut Node {
    ((*IChooser).AllocChooserNodeA)(IChooser, tag_list)
}

#[inline]
pub unsafe fn chooser_free_chooser_node(node: *mut Node) {
    ((*IChooser).FreeChooserNode)(IChooser, node)
}

#[inline]
pub unsafe fn chooser_set_chooser_node_attrs_a(node: *mut Node, tag_list: *mut TagItem) {
    ((*IChooser).SetChooserNodeAttrsA)(IChooser, node, tag_list)
}

#[inline]
pub unsafe fn chooser_get_chooser_node_attrs_a(node: *mut Node, tag_list: *mut TagItem) {
    ((*IChooser).GetChooserNodeAttrsA)(IChooser, node, tag_list)
}

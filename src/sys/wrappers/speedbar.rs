//! ISpeedBar global(s) and convenience wrappers.
//!
//! Hand-written ReAction toolbar binding with helper methods for
//! managing the toolbar's button list. Same shape as
//! chooser/clicktab/radiobutton.

use crate::sys::types::*;
use crate::sys::interfaces::speedbar::*;

// ---- ISpeedBar (SpeedBarIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static ISpeedBar: *mut SpeedBarIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut ISpeedBar: *mut SpeedBarIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn speedbar_speedbar_get_class() -> *mut APTR {
    ((*ISpeedBar).SPEEDBAR_GetClass)(ISpeedBar)
}

// ── Toolbar button-node lifecycle ────────────────────────────

#[inline]
pub unsafe fn speedbar_alloc_speed_button_node_a(id: u16, tag_list: *mut TagItem) -> *mut Node {
    ((*ISpeedBar).AllocSpeedButtonNodeA)(ISpeedBar, id, tag_list)
}

#[inline]
pub unsafe fn speedbar_free_speed_button_node(node: *mut Node) {
    ((*ISpeedBar).FreeSpeedButtonNode)(ISpeedBar, node)
}

#[inline]
pub unsafe fn speedbar_set_speed_button_node_attrs_a(node: *mut Node, tag_list: *mut TagItem) {
    ((*ISpeedBar).SetSpeedButtonNodeAttrsA)(ISpeedBar, node, tag_list)
}

#[inline]
pub unsafe fn speedbar_get_speed_button_node_attrs_a(node: *mut Node, tag_list: *mut TagItem) {
    ((*ISpeedBar).GetSpeedButtonNodeAttrsA)(ISpeedBar, node, tag_list)
}

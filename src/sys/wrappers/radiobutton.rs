//! IRadioButton global(s) and convenience wrappers.
//!
//! Hand-written ReAction radio-button binding with helper methods for
//! managing the radio group's node list. Same shape as
//! chooser/clicktab/speedbar.

use crate::sys::types::*;
use crate::sys::interfaces::radiobutton::*;

// ---- IRadioButton (RadioButtonIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IRadioButton: *mut RadioButtonIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IRadioButton: *mut RadioButtonIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn radiobutton_radiobutton_get_class() -> *mut APTR {
    ((*IRadioButton).RADIOBUTTON_GetClass)(IRadioButton)
}

// ── Radio-button node lifecycle ──────────────────────────────

#[inline]
pub unsafe fn radiobutton_alloc_radio_button_node_a(id: u16, tag_list: *mut TagItem) -> *mut Node {
    ((*IRadioButton).AllocRadioButtonNodeA)(IRadioButton, id, tag_list)
}

#[inline]
pub unsafe fn radiobutton_free_radio_button_node(node: *mut Node) {
    ((*IRadioButton).FreeRadioButtonNode)(IRadioButton, node)
}

#[inline]
pub unsafe fn radiobutton_set_radio_button_node_attrs_a(node: *mut Node, tag_list: *mut TagItem) {
    ((*IRadioButton).SetRadioButtonNodeAttrsA)(IRadioButton, node, tag_list)
}

#[inline]
pub unsafe fn radiobutton_get_radio_button_node_attrs_a(node: *mut Node, tag_list: *mut TagItem) {
    ((*IRadioButton).GetRadioButtonNodeAttrsA)(IRadioButton, node, tag_list)
}

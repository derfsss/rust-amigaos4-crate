//! IClickTab global(s) and convenience wrappers.
//!
//! Hand-written ReAction tab-gadget binding with helper methods for
//! managing the tab list.

use crate::sys::types::*;
use crate::sys::interfaces::clicktab::*;

// ---- IClickTab (ClickTabIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IClickTab: *mut ClickTabIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IClickTab: *mut ClickTabIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn clicktab_clicktab_get_class() -> *mut APTR {
    ((*IClickTab).CLICKTAB_GetClass)(IClickTab)
}

// ── Tab node lifecycle ───────────────────────────────────────

#[inline]
pub unsafe fn clicktab_alloc_click_tab_node_a(tag_list: *mut TagItem) -> *mut Node {
    ((*IClickTab).AllocClickTabNodeA)(IClickTab, tag_list)
}

#[inline]
pub unsafe fn clicktab_free_click_tab_node(node: *mut Node) {
    ((*IClickTab).FreeClickTabNode)(IClickTab, node)
}

#[inline]
pub unsafe fn clicktab_set_click_tab_node_attrs_a(node: *mut Node, tag_list: *mut TagItem) -> i32 {
    ((*IClickTab).SetClickTabNodeAttrsA)(IClickTab, node, tag_list)
}

#[inline]
pub unsafe fn clicktab_get_click_tab_node_attrs_a(node: *mut Node, tag_list: *mut TagItem) -> i32 {
    ((*IClickTab).GetClickTabNodeAttrsA)(IClickTab, node, tag_list)
}

#[inline]
pub unsafe fn clicktab_free_click_tab_list(list: *mut List) {
    ((*IClickTab).FreeClickTabList)(IClickTab, list)
}

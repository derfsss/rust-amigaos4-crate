//! IListBrowser global(s) and convenience wrappers.
//!
//! Hand-written binding for listbrowser.gadget — the largest
//! single-file ReAction widget at 16 regular methods. Manages
//! node-and-column-based hierarchical list display.

use crate::sys::types::*;
use crate::sys::interfaces::listbrowser::*;

// ---- IListBrowser (ListBrowserIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IListBrowser: *mut ListBrowserIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IListBrowser: *mut ListBrowserIFace = core::ptr::null_mut();

// ── Class accessor ───────────────────────────────────────────

#[inline]
pub unsafe fn listbrowser_listbrowser_get_class() -> *mut IClass {
    ((*IListBrowser).LISTBROWSER_GetClass)(IListBrowser)
}

// ── Node lifecycle ───────────────────────────────────────────

#[inline]
pub unsafe fn listbrowser_alloc_list_browser_node_a(num_columns: u16, tag_list: *mut TagItem) -> *mut Node {
    ((*IListBrowser).AllocListBrowserNodeA)(IListBrowser, num_columns, tag_list)
}

#[inline]
pub unsafe fn listbrowser_free_list_browser_node(node: *mut Node) {
    ((*IListBrowser).FreeListBrowserNode)(IListBrowser, node)
}

#[inline]
pub unsafe fn listbrowser_set_list_browser_node_attrs_a(node: *mut Node, tag_list: *mut TagItem) {
    ((*IListBrowser).SetListBrowserNodeAttrsA)(IListBrowser, node, tag_list)
}

#[inline]
pub unsafe fn listbrowser_get_list_browser_node_attrs_a(node: *mut Node, tag_list: *mut TagItem) {
    ((*IListBrowser).GetListBrowserNodeAttrsA)(IListBrowser, node, tag_list)
}

#[inline]
pub unsafe fn listbrowser_free_list_browser_list(list: *mut List) {
    ((*IListBrowser).FreeListBrowserList)(IListBrowser, list)
}

// ── Tree control (show/hide children) ────────────────────────

#[inline]
pub unsafe fn listbrowser_list_browser_select_all(list: *mut List) {
    ((*IListBrowser).ListBrowserSelectAll)(IListBrowser, list)
}

#[inline]
pub unsafe fn listbrowser_show_list_browser_node_children(node: *mut Node, depth: i16) {
    ((*IListBrowser).ShowListBrowserNodeChildren)(IListBrowser, node, depth)
}

#[inline]
pub unsafe fn listbrowser_hide_list_browser_node_children(node: *mut Node) {
    ((*IListBrowser).HideListBrowserNodeChildren)(IListBrowser, node)
}

#[inline]
pub unsafe fn listbrowser_show_all_list_browser_children(list: *mut List) {
    ((*IListBrowser).ShowAllListBrowserChildren)(IListBrowser, list)
}

#[inline]
pub unsafe fn listbrowser_hide_all_list_browser_children(list: *mut List) {
    ((*IListBrowser).HideAllListBrowserChildren)(IListBrowser, list)
}

#[inline]
pub unsafe fn listbrowser_list_browser_clear_all(list: *mut List) {
    ((*IListBrowser).ListBrowserClearAll)(IListBrowser, list)
}

// ── Column-info management ───────────────────────────────────

#[inline]
pub unsafe fn listbrowser_alloc_lbcolumn_info_a(num_columns: u16, tag_list: *mut TagItem) -> *mut ColumnInfo {
    ((*IListBrowser).AllocLBColumnInfoA)(IListBrowser, num_columns, tag_list)
}

#[inline]
pub unsafe fn listbrowser_set_lbcolumn_info_attrs_a(info: *mut ColumnInfo, tag_list: *mut TagItem) -> i32 {
    ((*IListBrowser).SetLBColumnInfoAttrsA)(IListBrowser, info, tag_list)
}

#[inline]
pub unsafe fn listbrowser_get_lbcolumn_info_attrs_a(info: *mut ColumnInfo, tag_list: *mut TagItem) -> i32 {
    ((*IListBrowser).GetLBColumnInfoAttrsA)(IListBrowser, info, tag_list)
}

#[inline]
pub unsafe fn listbrowser_free_lbcolumn_info(info: *mut ColumnInfo) {
    ((*IListBrowser).FreeLBColumnInfo)(IListBrowser, info)
}

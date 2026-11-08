//! ILayout global(s) and convenience wrappers.
//!
//! Hand-written binding for the ReAction layout engine — beyond the
//! basic `LAYOUT_GetClass` accessor, this exposes the engine
//! operations used to rethink/refresh window layouts and page-gadget
//! contents at runtime.

use crate::sys::types::*;
use crate::sys::interfaces::layout::*;

// ---- ILayout (LayoutIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static ILayout: *mut LayoutIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut ILayout: *mut LayoutIFace = core::ptr::null_mut();

// ── Class accessors ──────────────────────────────────────────

#[inline]
pub unsafe fn layout_layout_get_class() -> *mut APTR {
    ((*ILayout).LAYOUT_GetClass)(ILayout)
}

#[inline]
pub unsafe fn layout_page_get_class() -> *mut APTR {
    ((*ILayout).PAGE_GetClass)(ILayout)
}

// ── Layout-engine operations ─────────────────────────────────

#[inline]
pub unsafe fn layout_activate_layout_gadget(
    layout: *mut Gadget, win: *mut Window, req: *mut Requester, id: u32,
) -> u32 {
    ((*ILayout).ActivateLayoutGadget)(ILayout, layout, win, req, id)
}

#[inline]
pub unsafe fn layout_flush_layout_domain_cache(layout: *mut Gadget) {
    ((*ILayout).FlushLayoutDomainCache)(ILayout, layout)
}

#[inline]
pub unsafe fn layout_rethink_layout(
    layout: *mut Gadget, win: *mut Window, req: *mut Requester, refresh: u32,
) -> u32 {
    ((*ILayout).RethinkLayout)(ILayout, layout, win, req, refresh)
}

#[inline]
pub unsafe fn layout_layout_limits(
    layout: *mut Gadget, limits: *mut LayoutLimits, font: *mut TextFont, screen: *mut Screen,
) {
    ((*ILayout).LayoutLimits)(ILayout, layout, limits, font, screen)
}

// ── Page-gadget control ──────────────────────────────────────

#[inline]
pub unsafe fn layout_set_page_gadget_attrs_a(
    page: *mut Gadget, page_class: *mut APTR, win: *mut Window, req: *mut Requester, tag_list: *mut TagItem,
) -> u32 {
    ((*ILayout).SetPageGadgetAttrsA)(ILayout, page, page_class, win, req, tag_list)
}

#[inline]
pub unsafe fn layout_refresh_page_gadget(
    page: *mut Gadget, page_class: *mut APTR, win: *mut Window, req: *mut Requester,
) {
    ((*ILayout).RefreshPageGadget)(ILayout, page, page_class, win, req)
}

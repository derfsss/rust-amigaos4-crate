//! IPopupMenu global(s) and convenience wrappers.
//!
//! Hand-written to match the amigaos4-bindgen convention used by the
//! other wrapper modules.
//!
//! Note: PopupMenuIFace's methods are named `POPUPMENU_<Method>` in the
//! SDK header, so the bindgen snake_case rule produces wrapper names
//! with a doubled prefix — `popupmenu_popupmenu_get_class` etc. Ugly
//! but mechanically consistent with how the audit derives names from
//! the vtable struct.

use crate::sys::types::*;
use crate::sys::interfaces::popupmenu::*;

// ---- IPopupMenu (PopupMenuIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IPopupMenu: *mut PopupMenuIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IPopupMenu: *mut PopupMenuIFace = core::ptr::null_mut();

// ── Class lookup ─────────────────────────────────────────────

#[inline]
pub unsafe fn popupmenu_popupmenu_get_class() -> *mut APTR {
    ((*IPopupMenu).POPUPMENU_GetClass)(IPopupMenu)
}

#[inline]
pub unsafe fn popupmenu_popupmenu_get_item_class() -> *mut APTR {
    ((*IPopupMenu).POPUPMENU_GetItemClass)(IPopupMenu)
}

// ── ID-list construction (radio/MX menus) ────────────────────

#[inline]
pub unsafe fn popupmenu_popupmenu_make_mxlist_a(ids: *mut u32) -> APTR {
    ((*IPopupMenu).POPUPMENU_MakeMXListA)(IPopupMenu, ids)
}

#[inline]
pub unsafe fn popupmenu_popupmenu_free_idlist(idlist: APTR) {
    ((*IPopupMenu).POPUPMENU_FreeIDList)(IPopupMenu, idlist)
}

#[inline]
pub unsafe fn popupmenu_popupmenu_make_idlist_a(tags: *mut TagItem) -> APTR {
    ((*IPopupMenu).POPUPMENU_MakeIDListA)(IPopupMenu, tags)
}

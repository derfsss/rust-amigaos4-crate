//! Pop-up menus (popupmenu.class), safe (feature `popupmenu`, with
//! `intuition`).
//!
//! A [`PopupMenu`] is built from the same [`Item`]s as a menu strip
//! (actions, toggles, separators, submenus; a radio item shows a check
//! mark but excludes nothing here) and shown with [`PopupMenu::open`],
//! which waits until the user picks an item or lets go, and returns the
//! item's id. The objects and their texts are kept until the menu is
//! dropped (DisposeObject frees the items with the menu).

use alloc::vec::Vec;
use core::sync::atomic::{AtomicPtr, Ordering};

use crate::cstr::c_copy;
use crate::error::{AmigaError, Result};
use crate::menu::{Item, ItemKind, NO_MENU_ID};
use crate::sys::{self, IntuitionIFace, PopupMenuIFace, TagItem, APTR, TAG_DONE};

// classes/popupmenu.h (tests/layout/popup.c): POPUPMENU_Dummy =
// REACTION_Dummy + 0x25900.
const PM_DUMMY: u32 = 0x8000_0000 + 0x0502_5900;
const PMA_ADD_ITEM: u32 = PM_DUMMY + 1;
const PMIA_TITLE: u32 = PM_DUMMY + 50;
const PMIA_ID: u32 = PM_DUMMY + 52;
const PMIA_SUB_MENU: u32 = PM_DUMMY + 53;
const PMIA_CHECK_IT: u32 = PM_DUMMY + 56;
const PMIA_CHECKED: u32 = PM_DUMMY + 57;
const PMIA_DISABLED: u32 = PM_DUMMY + 61;
const PMIA_COMM_KEY: u32 = PM_DUMMY + 67;
const PMIA_TOGGLE: u32 = PM_DUMMY + 75;
const PMIA_WIDE_TITLE_BAR: u32 = PM_DUMMY + 80;
const PM_OPEN: u32 = 0x0057_1000;

static MENU_CLASS: AtomicPtr<APTR> = AtomicPtr::new(core::ptr::null_mut());
static ITEM_CLASS: AtomicPtr<APTR> = AtomicPtr::new(core::ptr::null_mut());

/// popupmenu.class's two classes, opened once (OpenClass, then its
/// interface for the item class) and kept.
fn classes(i: *mut IntuitionIFace) -> Result<(*mut APTR, *mut APTR)> {
    let (m, it) = (MENU_CLASS.load(Ordering::Acquire), ITEM_CLASS.load(Ordering::Acquire));
    if !m.is_null() && !it.is_null() {
        return Ok((m, it));
    }
    let mut class: *mut APTR = core::ptr::null_mut();
    // SAFETY: an open interface, a NUL-terminated name and a pointer to
    // write the class to.
    let lib = unsafe { ((*i).OpenClass)(i, c"popupmenu.class".as_ptr() as *const u8, 50, &mut class) };
    if lib.is_null() || class.is_null() {
        return Err(AmigaError::Unsupported("popupmenu.class could not be opened"));
    }
    // SAFETY: the class library just opened; its "main" interface v1
    // gives the item class. Kept for the program's life, like the class.
    let item = unsafe {
        let pm = sys::exec_get_interface(lib as *mut sys::Library, c"main".as_ptr() as *const u8, 1, core::ptr::null()) as *mut PopupMenuIFace;
        if pm.is_null() {
            return Err(AmigaError::Unsupported("popupmenu.class has no main interface"));
        }
        ((*pm).POPUPMENU_GetItemClass)(pm)
    };
    if item.is_null() {
        return Err(AmigaError::Unsupported("popupmenu.class gave no item class"));
    }
    MENU_CLASS.store(class, Ordering::Release);
    ITEM_CLASS.store(item, Ordering::Release);
    Ok((class, item))
}

/// A pop-up menu; DisposeObject on drop.
pub struct PopupMenu {
    i: *mut IntuitionIFace,
    menu: *mut APTR,
    _strings: Vec<Vec<u8>>,
}

struct Builder {
    i: *mut IntuitionIFace,
    menu_class: *mut APTR,
    item_class: *mut APTR,
    strings: Vec<Vec<u8>>,
}

impl Builder {
    fn text(&mut self, s: &str) -> Result<u32> {
        let c = c_copy(s.as_bytes())?;
        let p = c.as_ptr() as u32;
        self.strings.push(c);
        Ok(p)
    }

    fn new_object(&self, class: *mut APTR, tags: &[TagItem]) -> Result<*mut APTR> {
        // SAFETY: an open interface, a popupmenu class and a TAG_DONE-ended
        // list whose strings live in the builder (then the menu).
        let o = unsafe { ((*self.i).NewObjectA)(self.i, class, core::ptr::null_mut(), tags.as_ptr()) };
        if o.is_null() { Err(AmigaError::AllocationFailed) } else { Ok(o) }
    }

    fn dispose(&self, o: *mut APTR) {
        // SAFETY: an object made here that no menu has taken.
        unsafe { ((*self.i).DisposeObject)(self.i, o) };
    }

    /// A menu object holding `items` (a submenu or the top).
    fn menu(&mut self, items: &[Item]) -> Result<*mut APTR> {
        let mut made = Vec::new();
        for it in items {
            match self.item(it) {
                Ok(o) => made.push(o),
                Err(e) => {
                    for o in made {
                        self.dispose(o);
                    }
                    return Err(e);
                }
            }
        }
        let mut t: Vec<TagItem> = made.iter().map(|o| TagItem { ti_Tag: PMA_ADD_ITEM, ti_Data: *o as u32 }).collect();
        t.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        self.new_object(self.menu_class, &t).inspect_err(|_| {
            for o in made {
                self.dispose(o);
            }
        })
    }

    fn item(&mut self, it: &Item) -> Result<*mut APTR> {
        let mut t: Vec<TagItem> = Vec::new();
        if it.kind() == ItemKind::Separator {
            t.push(TagItem { ti_Tag: PMIA_WIDE_TITLE_BAR, ti_Data: 1 });
        } else {
            t.push(TagItem { ti_Tag: PMIA_TITLE, ti_Data: self.text(it.label())? });
            if it.id() != NO_MENU_ID {
                t.push(TagItem { ti_Tag: PMIA_ID, ti_Data: it.id() });
            }
            if let Some(k) = it.key_text() {
                t.push(TagItem { ti_Tag: PMIA_COMM_KEY, ti_Data: self.text(k)? });
            }
            t.push(TagItem { ti_Tag: PMIA_DISABLED, ti_Data: it.is_disabled() as u32 });
            if matches!(it.kind(), ItemKind::Toggle | ItemKind::Radio) {
                t.push(TagItem { ti_Tag: PMIA_CHECK_IT, ti_Data: 1 });
                t.push(TagItem { ti_Tag: PMIA_TOGGLE, ti_Data: (it.kind() == ItemKind::Toggle) as u32 });
                t.push(TagItem { ti_Tag: PMIA_CHECKED, ti_Data: it.is_checked() as u32 });
            }
            if !it.children().is_empty() {
                let sub = self.menu(it.children())?;
                t.push(TagItem { ti_Tag: PMIA_SUB_MENU, ti_Data: sub as u32 });
            }
        }
        t.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        self.new_object(self.item_class, &t)
    }
}

impl PopupMenu {
    /// Build the menu from `items`.
    pub fn new(items: &[Item]) -> Result<Self> {
        if items.is_empty() {
            return Err(AmigaError::InvalidArgument("a pop-up menu needs items"));
        }
        let i = crate::intuition::intuition_iface()?;
        let (menu_class, item_class) = classes(i)?;
        let mut b = Builder { i, menu_class, item_class, strings: Vec::new() };
        let menu = b.menu(items)?;
        Ok(Self { i, menu, _strings: b.strings })
    }

    /// Show the menu at the mouse over `window` and wait (PM_OPEN): the
    /// id of the item picked, or None when nothing was. `window` must be
    /// an open window (e.g. `intuition::Window::as_ptr`).
    pub fn open(&self, window: *mut sys::Window) -> Option<u32> {
        if window.is_null() {
            return None;
        }
        let msg = [PM_OPEN, window as u32];
        // SAFETY: our menu object and a pmOpen message (MethodID, Window)
        // for the caller's open window, alive for the call.
        let item = unsafe { ((*self.i).IDoMethodA)(self.i, self.menu, msg.as_ptr() as APTR) } as *mut APTR;
        if item.is_null() {
            return None;
        }
        let mut id = 0u32;
        // SAFETY: the item object PM_OPEN returned (one of our menu's) and
        // a u32 to write.
        let ok = unsafe { ((*self.i).GetAttr)(self.i, PMIA_ID, item, &mut id) } != 0;
        (ok && id != NO_MENU_ID).then_some(id)
    }
}

impl Drop for PopupMenu {
    fn drop(&mut self) {
        // SAFETY: our menu, which disposes its items and submenus.
        unsafe { ((*self.i).DisposeObject)(self.i, self.menu) };
    }
}

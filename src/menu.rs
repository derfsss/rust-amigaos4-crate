//! BOOPSI menus (menuclass, Intuition V54), safe (feature `intuition`).
//!
//! A menu is described with [`Strip`] (the titles in the screen's bar),
//! [`Menu`] (one title and its items) and [`Item`] (an action, a toggle,
//! a radio choice, a separator, or a submenu), then built into a
//! [`MenuTree`]: the menuclass object tree, with every label and key it
//! points at kept alongside, disposed on drop.
//!
//! Items carry the application's own ids. After a menu event the chosen
//! ids come from [`MenuTree::selected`] (MM_NEXTSELECT), and the state of
//! a toggle or radio item is read and set by id ([`checked`](MenuTree::checked),
//! [`set_checked`](MenuTree::set_checked); MM_GETSTATE/MM_SETSTATE), as are
//! [`disabled`](MenuTree::disabled) items. menuclass keeps itself in step
//! with Intuition: changes show at once, attached or not.
//!
//! A window takes a tree with `intuition::Window::set_menu_strip` or
//! `set_context_menu` (shown by the right button over the window's
//! inside), or a ReAction `WindowSpec::menu` / `Gadget::context_menu`; it
//! owns the tree and reports picks as menu events.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::cstr::c_copy;
use crate::error::{AmigaError, Result};
use crate::sys::{IntuitionIFace, TagItem, APTR, TAG_DONE};

// intuition/menuclass.h (tests/layout/menu.c).
const MA_DUMMY: u32 = 0x8000_0000 + 0x0044_0000;
const MA_TYPE: u32 = MA_DUMMY + 1;
const MA_LABEL: u32 = MA_DUMMY + 2;
const MA_SEPARATOR: u32 = MA_DUMMY + 4;
const MA_ID: u32 = MA_DUMMY + 5;
const MA_KEY: u32 = MA_DUMMY + 6;
const MA_DISABLED: u32 = MA_DUMMY + 8;
const MA_TOGGLE: u32 = MA_DUMMY + 9;
const MA_MX: u32 = MA_DUMMY + 10;
const MA_SELECTED: u32 = MA_DUMMY + 11;
const MA_ADD_CHILD: u32 = MA_DUMMY + 50;
const T_ROOT: u32 = -1i32 as u32;
const T_MENU: u32 = 0;
const T_ITEM: u32 = 1;
const MM_FINDID: u32 = 4001;
const MM_SETSTATE: u32 = 4006;
const MM_GETSTATE: u32 = 4007;
const MM_NEXTSELECT: u32 = 4009;
const MS_CHECKED: u32 = 1;
const MS_DISABLED: u32 = 2;
/// The "no item" id.
pub const NO_MENU_ID: u32 = 0;

/// What an item does.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Kind {
    Action,
    Toggle,
    /// A radio choice: excludes the other radio items of the same group
    /// in the same panel.
    Radio(u32),
    Separator,
}

/// One menu item (or submenu, when it has items of its own).
#[derive(Clone, Debug)]
pub struct Item {
    kind: Kind,
    id: u32,
    label: String,
    key: Option<String>,
    checked: bool,
    disabled: bool,
    items: Vec<Item>,
}

impl Item {
    fn new(kind: Kind, id: u32, label: &str) -> Self {
        Self { kind, id, label: label.into(), key: None, checked: false, disabled: false, items: Vec::new() }
    }

    /// An item that does something; `id` (non-zero) is what a pick
    /// reports.
    pub fn action(id: u32, label: &str) -> Self {
        Self::new(Kind::Action, id, label)
    }

    /// An option switched on and off (a check mark).
    pub fn toggle(id: u32, label: &str) -> Self {
        Self::new(Kind::Toggle, id, label)
    }

    /// One of a set of choices: picking it clears the other radio items
    /// of `group` in the same panel.
    pub fn radio(id: u32, label: &str, group: u32) -> Self {
        Self::new(Kind::Radio(group), id, label)
    }

    /// A line between items.
    pub fn separator() -> Self {
        Self::new(Kind::Separator, NO_MENU_ID, "")
    }

    /// A submenu: an item with items of its own (any depth).
    pub fn submenu(label: &str, items: Vec<Item>) -> Self {
        let mut s = Self::new(Kind::Action, NO_MENU_ID, label);
        s.items = items;
        s
    }

    /// The keyboard shortcut with the Amiga key ("O"); a longer text is
    /// shown but not handled by Intuition.
    pub fn key(mut self, k: &str) -> Self {
        self.key = Some(k.into());
        self
    }

    /// A toggle or radio item starting checked.
    pub fn checked(mut self, on: bool) -> Self {
        self.checked = on;
        self
    }

    /// Shown greyed out and not selectable.
    pub fn disabled(mut self, on: bool) -> Self {
        self.disabled = on;
        self
    }

    #[cfg_attr(not(feature = "popupmenu"), allow(dead_code))]
    pub(crate) fn kind(&self) -> ItemKind {
        match self.kind {
            Kind::Action => ItemKind::Action,
            Kind::Toggle => ItemKind::Toggle,
            Kind::Radio(_) => ItemKind::Radio,
            Kind::Separator => ItemKind::Separator,
        }
    }

    #[cfg_attr(not(feature = "popupmenu"), allow(dead_code))]
    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    #[cfg_attr(not(feature = "popupmenu"), allow(dead_code))]
    pub(crate) fn id(&self) -> u32 {
        self.id
    }

    #[cfg_attr(not(feature = "popupmenu"), allow(dead_code))]
    pub(crate) fn key_text(&self) -> Option<&str> {
        self.key.as_deref()
    }

    #[cfg_attr(not(feature = "popupmenu"), allow(dead_code))]
    pub(crate) fn is_checked(&self) -> bool {
        self.checked
    }

    #[cfg_attr(not(feature = "popupmenu"), allow(dead_code))]
    pub(crate) fn is_disabled(&self) -> bool {
        self.disabled
    }

    #[cfg_attr(not(feature = "popupmenu"), allow(dead_code))]
    pub(crate) fn children(&self) -> &[Item] {
        &self.items
    }
}

/// An item's kind, for other menu builders (popupmenu.class).
#[cfg_attr(not(feature = "popupmenu"), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ItemKind {
    Action,
    Toggle,
    Radio,
    Separator,
}

/// One title in the menu bar and its items.
#[derive(Clone, Debug)]
pub struct Menu {
    title: String,
    id: u32,
    disabled: bool,
    items: Vec<Item>,
}

impl Menu {
    pub fn new(title: &str) -> Self {
        Self { title: title.into(), id: NO_MENU_ID, disabled: false, items: Vec::new() }
    }

    pub fn item(mut self, it: Item) -> Self {
        self.items.push(it);
        self
    }

    pub fn separator(self) -> Self {
        self.item(Item::separator())
    }

    /// The whole menu greyed out.
    pub fn disabled(mut self, on: bool) -> Self {
        self.disabled = on;
        self
    }
}

/// A menu bar: the menus in order. A context menu is a strip of one menu.
#[derive(Clone, Debug, Default)]
pub struct Strip {
    menus: Vec<Menu>,
}

impl Strip {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn menu(mut self, m: Menu) -> Self {
        self.menus.push(m);
        self
    }
}

/// A built menuclass tree; DisposeObject on drop (after any window has
/// let go of it: the windows that take one own it).
pub struct MenuTree {
    i: *mut IntuitionIFace,
    root: *mut APTR,
    _strings: Vec<Vec<u8>>,
    ids: Vec<u32>,
}

struct Builder {
    i: *mut IntuitionIFace,
    strings: Vec<Vec<u8>>,
    ids: Vec<u32>,
}

impl Builder {
    fn text(&mut self, s: &str) -> Result<u32> {
        let c = c_copy(s.as_bytes())?;
        // The Vec's heap block does not move when `strings` grows.
        let p = c.as_ptr() as u32;
        self.strings.push(c);
        Ok(p)
    }

    fn object(&self, tags: &[TagItem]) -> Result<*mut APTR> {
        // SAFETY: an open interface, menuclass by name (a public class built
        // into Intuition V54) and a TAG_DONE-ended list whose strings live
        // in the builder (then the tree).
        let o = unsafe { ((*self.i).NewObjectA)(self.i, core::ptr::null_mut(), c"menuclass".as_ptr() as APTR, tags.as_ptr()) };
        if o.is_null() { Err(AmigaError::Unsupported("menuclass refused an object (Intuition V54 needed)")) } else { Ok(o) }
    }

    fn dispose(&self, o: *mut APTR) {
        // SAFETY: an object made by `object` that no parent has taken.
        unsafe { ((*self.i).DisposeObject)(self.i, o) };
    }

    /// The items of one panel: a NULL-free list of objects, or every one
    /// made here disposed on failure.
    fn panel(&mut self, items: &[Item]) -> Result<Vec<*mut APTR>> {
        let mut made = Vec::new();
        for (k, it) in items.iter().enumerate() {
            match self.item(it, items, k) {
                Ok(o) => made.push(o),
                Err(e) => {
                    for o in made {
                        self.dispose(o);
                    }
                    return Err(e);
                }
            }
        }
        Ok(made)
    }

    fn item(&mut self, it: &Item, panel: &[Item], index: usize) -> Result<*mut APTR> {
        let mut t: Vec<TagItem> = vec![TagItem { ti_Tag: MA_TYPE, ti_Data: T_ITEM }];
        if it.kind == Kind::Separator {
            t.push(TagItem { ti_Tag: MA_SEPARATOR, ti_Data: 1 });
        } else {
            t.push(TagItem { ti_Tag: MA_LABEL, ti_Data: self.text(&it.label)? });
            if it.id != NO_MENU_ID {
                t.push(TagItem { ti_Tag: MA_ID, ti_Data: it.id });
                self.ids.push(it.id);
            }
            if let Some(k) = &it.key {
                t.push(TagItem { ti_Tag: MA_KEY, ti_Data: self.text(k)? });
            }
            t.push(TagItem { ti_Tag: MA_DISABLED, ti_Data: it.disabled as u32 });
            match it.kind {
                Kind::Toggle => {
                    t.push(TagItem { ti_Tag: MA_TOGGLE, ti_Data: 1 });
                    t.push(TagItem { ti_Tag: MA_SELECTED, ti_Data: it.checked as u32 });
                }
                Kind::Radio(g) => {
                    // Bit n stands for the panel's n-th item: exclude the
                    // other radio items of this group.
                    let mask = panel
                        .iter()
                        .enumerate()
                        .filter(|(n, o)| *n != index && *n < 32 && o.kind == Kind::Radio(g))
                        .fold(0u32, |m, (n, _)| m | (1 << n));
                    t.push(TagItem { ti_Tag: MA_MX, ti_Data: mask });
                    t.push(TagItem { ti_Tag: MA_SELECTED, ti_Data: it.checked as u32 });
                }
                _ => {}
            }
        }
        let children = self.panel(&it.items)?;
        for c in &children {
            t.push(TagItem { ti_Tag: MA_ADD_CHILD, ti_Data: *c as u32 });
        }
        t.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        self.object(&t).inspect_err(|_| {
            for c in children {
                self.dispose(c);
            }
        })
    }

    fn menu(&mut self, m: &Menu) -> Result<*mut APTR> {
        let mut t: Vec<TagItem> = vec![
            TagItem { ti_Tag: MA_TYPE, ti_Data: T_MENU },
            TagItem { ti_Tag: MA_LABEL, ti_Data: self.text(&m.title)? },
            TagItem { ti_Tag: MA_DISABLED, ti_Data: m.disabled as u32 },
        ];
        if m.id != NO_MENU_ID {
            t.push(TagItem { ti_Tag: MA_ID, ti_Data: m.id });
        }
        let children = self.panel(&m.items)?;
        for c in &children {
            t.push(TagItem { ti_Tag: MA_ADD_CHILD, ti_Data: *c as u32 });
        }
        t.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        self.object(&t).inspect_err(|_| {
            for c in children {
                self.dispose(c);
            }
        })
    }
}

impl MenuTree {
    /// Build the menuclass objects for `strip`.
    pub fn new(strip: &Strip) -> Result<Self> {
        if strip.menus.is_empty() || strip.menus.iter().any(|m| m.items.is_empty()) {
            return Err(AmigaError::InvalidArgument("a menu strip needs menus, each with items"));
        }
        let i = crate::intuition::intuition_iface()?;
        let mut b = Builder { i, strings: Vec::new(), ids: Vec::new() };
        let mut menus = Vec::new();
        for m in &strip.menus {
            match b.menu(m) {
                Ok(o) => menus.push(o),
                Err(e) => {
                    for o in menus {
                        b.dispose(o);
                    }
                    return Err(e);
                }
            }
        }
        let mut t: Vec<TagItem> = vec![TagItem { ti_Tag: MA_TYPE, ti_Data: T_ROOT }];
        for o in &menus {
            t.push(TagItem { ti_Tag: MA_ADD_CHILD, ti_Data: *o as u32 });
        }
        t.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        let root = match b.object(&t) {
            Ok(r) => r,
            Err(e) => {
                for o in menus {
                    b.dispose(o);
                }
                return Err(e);
            }
        };
        Ok(Self { i, root, _strings: b.strings, ids: b.ids })
    }

    /// The root object, for WA_MenuStrip, WINDOW_MenuStrip or
    /// GA_ContextMenu.
    pub fn as_ptr(&self) -> *mut APTR {
        self.root
    }

    fn method(&self, msg: &[u32]) -> u32 {
        // SAFETY: our root object and a method message of the layout
        // menuclass.h gives (MethodID, reserved, arguments), alive for the
        // call.
        unsafe { ((*self.i).IDoMethodA)(self.i, self.root, msg.as_ptr() as APTR) }
    }

    /// Whether an item with this id is in the tree.
    pub fn contains(&self, id: u32) -> bool {
        id != NO_MENU_ID && self.ids.contains(&id) && self.method(&[MM_FINDID, 0, id]) != 0
    }

    fn state(&self, id: u32) -> Option<u32> {
        self.contains(id).then(|| self.method(&[MM_GETSTATE, 0, id]))
    }

    fn set_state(&self, id: u32, bit: u32, on: bool) -> bool {
        if !self.contains(id) {
            return false;
        }
        self.method(&[MM_SETSTATE, 0, id, bit, if on { bit } else { 0 }]);
        true
    }

    /// A toggle or radio item's check mark; None for an unknown id.
    pub fn checked(&self, id: u32) -> Option<bool> {
        self.state(id).map(|s| s & MS_CHECKED != 0)
    }

    /// Set or clear a check mark (the same as the user picking it, without
    /// a menu event). False for an unknown id.
    pub fn set_checked(&self, id: u32, on: bool) -> bool {
        self.set_state(id, MS_CHECKED, on)
    }

    pub fn disabled(&self, id: u32) -> Option<bool> {
        self.state(id).map(|s| s & MS_DISABLED != 0)
    }

    pub fn set_disabled(&self, id: u32, on: bool) -> bool {
        self.set_state(id, MS_DISABLED, on)
    }

    /// The ids picked in the last menu session (MM_NEXTSELECT), in order.
    pub fn selected(&self) -> Vec<u32> {
        selected_of(self.i, self.root)
    }

    /// Whether `obj` is this tree's root.
    pub(crate) fn is(&self, obj: *mut APTR) -> bool {
        obj == self.root
    }
}

/// MM_NEXTSELECT over the tree rooted at `root` (a strip or context
/// menu): the ids picked, at most 64.
pub(crate) fn selected_of(i: *mut IntuitionIFace, root: *mut APTR) -> Vec<u32> {
    let mut out = Vec::new();
    if root.is_null() {
        return out;
    }
    let mut id = NO_MENU_ID;
    loop {
        let msg = [MM_NEXTSELECT, 0, id];
        // SAFETY: a live menuclass root (the caller's) and an MM_NEXTSELECT
        // message alive for the call.
        id = unsafe { ((*i).IDoMethodA)(i, root, msg.as_ptr() as APTR) };
        if id == NO_MENU_ID || out.len() >= 64 || out.contains(&id) {
            return out;
        }
        out.push(id);
    }
}

/// The checked state of `id` in the tree rooted at `root`.
pub(crate) fn checked_of(i: *mut IntuitionIFace, root: *mut APTR, id: u32) -> bool {
    let msg = [MM_GETSTATE, 0, id];
    // SAFETY: a live menuclass root and an MM_GETSTATE message.
    unsafe { ((*i).IDoMethodA)(i, root, msg.as_ptr() as APTR) & MS_CHECKED != 0 }
}

impl Drop for MenuTree {
    fn drop(&mut self) {
        // SAFETY: our root, which disposes its children; no window has it
        // (the windows that take a tree own it and let go first).
        unsafe { ((*self.i).DisposeObject)(self.i, self.root) };
    }
}

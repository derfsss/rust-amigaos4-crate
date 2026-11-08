//! Windows on a public screen, their events, and requesters, safe
//! (feature `intuition`).
//!
//! - [`PubScreen`]: LockPubScreen (the default public screen or one by
//!   name), UnlockPubScreen on drop; its size and title.
//! - [`Window`]: OpenWindowTagList on a locked public screen, CloseWindow
//!   on drop. The title strings live in the window. [`Window::next`] takes
//!   an IntuiMessage off the window's port, copies it into an [`Event`]
//!   and replies it before returning: no Intuition message is held by the
//!   program. Geometry is read back from the window; move, size and
//!   activate calls are carried out later by Intuition (wait for
//!   [`Event::ChangeWindow`] / [`Event::NewSize`]).
//! - [`Window::rastport`] (feature `graphics` too): the window's RastPort,
//!   borrowed, with every coordinate checked against the window; its font
//!   is put back when the borrow ends.
//! - [`easy_request`] and [`Requester`] (EasyRequestArgs, and the
//!   non-blocking BuildEasyRequestArgs / SysReqHandler / FreeSysRequest):
//!   the texts are shown as given (a `%` is doubled, since they are format
//!   strings), the answer is the button's index from the left.
//!
//! - [`MenuStrip`] (feature `gadtools` too): a menu built with GadTools
//!   from [`MenuEntry`]s, attached with [`Window::set_menu`]; a choice
//!   arrives as [`Event::Menu`] with the entry's own id (every item of a
//!   multiple selection, in order).
//! - [`Window::refresh`]: BeginRefresh / EndRefresh around redrawing a
//!   simple-refresh window after [`Event::RefreshWindow`].
//! - [`Window::clip`] (feature `layers`): drawing limited to a region
//!   (InstallClipRegion, the old region put back on drop).
//!
//! Not wrapped yet: gadgets and BOOPSI, screens of the program's own,
//! Workbench start-up.

use alloc::string::String;
use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::cstr::c_copy;
use crate::error::{AmigaError, Result};
use crate::sys::{self, IntuitionIFace, TagItem, TAG_DONE, TAG_USER};

crate::iface::iface!(iface, IntuitionIFace, IIntuition, b"intuition.library\0", b"main\0", 50);

#[cfg_attr(not(feature = "datatypes"), allow(dead_code))]
pub(crate) fn intuition_iface() -> Result<*mut IntuitionIFace> {
    iface()
}

/// IDCMP classes (intuition/intuition.h).
pub mod idcmp {
    pub const NEWSIZE: u32 = 0x0000_0002;
    pub const REFRESHWINDOW: u32 = 0x0000_0004;
    pub const MOUSEBUTTONS: u32 = 0x0000_0008;
    pub const MOUSEMOVE: u32 = 0x0000_0010;
    pub const CLOSEWINDOW: u32 = 0x0000_0200;
    pub const RAWKEY: u32 = 0x0000_0400;
    pub const ACTIVEWINDOW: u32 = 0x0004_0000;
    pub const INACTIVEWINDOW: u32 = 0x0008_0000;
    pub const VANILLAKEY: u32 = 0x0020_0000;
    pub const INTUITICKS: u32 = 0x0040_0000;
    pub const CHANGEWINDOW: u32 = 0x0200_0000;
    pub const MENUPICK: u32 = 0x0000_0100;
}

/// Mouse button codes and qualifier bits.
pub mod code {
    pub const SELECTDOWN: u16 = 104;
    pub const SELECTUP: u16 = 232;
    pub const MENUDOWN: u16 = 105;
    pub const MENUUP: u16 = 233;
    pub const QUAL_LSHIFT: u16 = 1;
    pub const QUAL_RSHIFT: u16 = 2;
    pub const QUAL_CONTROL: u16 = 8;
    pub const QUAL_LALT: u16 = 16;
}

// WA_* tags (TAG_USER + 99 + n).
const WA_LEFT: u32 = TAG_USER + 100;
const WA_TOP: u32 = TAG_USER + 101;
const WA_IDCMP: u32 = TAG_USER + 106;
const WA_TITLE: u32 = TAG_USER + 110;
const WA_MIN_WIDTH: u32 = TAG_USER + 114;
const WA_MIN_HEIGHT: u32 = TAG_USER + 115;
const WA_MAX_WIDTH: u32 = TAG_USER + 116;
const WA_MAX_HEIGHT: u32 = TAG_USER + 117;
const WA_INNER_WIDTH: u32 = TAG_USER + 118;
const WA_INNER_HEIGHT: u32 = TAG_USER + 119;
const WA_PUB_SCREEN: u32 = TAG_USER + 121;
const WA_SIZE_GADGET: u32 = TAG_USER + 129;
const WA_DRAG_BAR: u32 = TAG_USER + 130;
const WA_DEPTH_GADGET: u32 = TAG_USER + 131;
const WA_CLOSE_GADGET: u32 = TAG_USER + 132;
const WA_NO_CARE_REFRESH: u32 = TAG_USER + 135;
const WA_ACTIVATE: u32 = TAG_USER + 137;
const WA_RMB_TRAP: u32 = TAG_USER + 138;
const WA_SMART_REFRESH: u32 = TAG_USER + 141;
const WA_SIMPLE_REFRESH: u32 = TAG_USER + 140;
/// New-look menus: the screen's menu pens and font, not the window's
/// old-style pens (menuclass menus come out black without it).
const WA_NEW_LOOK_MENUS: u32 = TAG_USER + 99 + 0x30;
const WA_BUSY_POINTER: u32 = TAG_USER + 152;
const WA_POINTER_DELAY: u32 = TAG_USER + 153;
const WA_CONTEXT_MENU_HOOK: u32 = TAG_USER + 99 + 0x52;
/// struct ContextMenuMsg: State 0, Menu 4, Context 8.
const CMM_MENU: usize = 4;
/// MENUNULL; struct MenuItem: Flags at 12, NextSelect at 32, 34 bytes,
/// GadTools' user data right after it.
const MENUNULL: u16 = 0xFFFF;
const MI_FLAGS: usize = 12;
const MI_NEXT_SELECT: usize = 32;
const MI_USERDATA: usize = 34;
/// struct Menu: NextMenu 0, FirstItem 18; struct MenuItem: NextItem 0,
/// SubItem 28.
#[cfg_attr(not(feature = "gadtools"), allow(dead_code))]
const MENU_FIRST_ITEM: usize = 18;
#[cfg_attr(not(feature = "gadtools"), allow(dead_code))]
const MI_SUB_ITEM: usize = 28;
const CHECKED: u16 = 0x0100;

/// struct Window, struct Screen and struct IntuiMessage offsets
/// (tests/layout/intuition.c). Window's pointers are not 4-aligned.
const WIN_LEFT: usize = 4;
const WIN_TOP: usize = 6;
const WIN_WIDTH: usize = 8;
const WIN_HEIGHT: usize = 10;
const WIN_MOUSE_Y: usize = 12;
const WIN_MOUSE_X: usize = 14;
#[cfg_attr(not(any(feature = "graphics", feature = "layers")), allow(dead_code))]
const WIN_RPORT: usize = 50;
const WIN_BORDER_LEFT: usize = 54;
const WIN_BORDER_TOP: usize = 55;
const WIN_BORDER_RIGHT: usize = 56;
const WIN_BORDER_BOTTOM: usize = 57;
const WIN_USERPORT: usize = 86;
const SCR_WIDTH: usize = 12;
const SCR_HEIGHT: usize = 14;
const SCR_TITLE: usize = 22;
const IM_CLASS: usize = 20;
const IM_IADDRESS: usize = 28;
const IM_CODE: usize = 24;
const IM_QUALIFIER: usize = 26;
const IM_MOUSE_X: usize = 32;
const IM_MOUSE_Y: usize = 34;
const IM_SECONDS: usize = 36;
const IM_MICROS: usize = 40;
/// struct MsgPort's mp_SigBit.
const MP_SIGBIT: usize = 15;

fn rd<T: Copy>(p: *const u8, off: usize) -> T {
    // SAFETY: callers pass a live Intuition structure and a field offset
    // from its header (tests/layout/intuition.c).
    unsafe { core::ptr::read_unaligned(p.add(off) as *const T) }
}

/// A locked public screen; UnlockPubScreen on drop. Windows opened on it
/// borrow it.
pub struct PubScreen {
    i: *mut IntuitionIFace,
    scr: *mut sys::Screen,
    _t: crate::track::Tracked,
    _p: PhantomData<*const ()>,
}

/// Unlock a registered public screen (the panic path; see `crate::track`).
unsafe fn release_screen(i: usize, scr: usize) {
    let i = i as *mut IntuitionIFace;
    // SAFETY: the registry calls this once, for a screen still locked;
    // the windows on it were registered later, so they are closed first.
    unsafe { ((*i).UnlockPubScreen)(i, core::ptr::null(), scr as *mut sys::Screen) };
}

/// Remove a registered AppWindow.
#[cfg(feature = "workbench")]
unsafe fn release_app_window(aw: usize, _: usize) {
    crate::workbench::remove_app_window(aw as *mut sys::AppWindow);
}

/// Close a registered window: its menu strip cleared first.
unsafe fn release_window(i: usize, w: usize) {
    let (i, w) = (i as *mut IntuitionIFace, w as *mut sys::Window);
    // SAFETY: the registry calls this once, for a window still open; an
    // AppWindow on it was registered later and is removed first.
    unsafe {
        ((*i).ClearMenuStrip)(i, w);
        ((*i).CloseWindow)(i, w);
    }
}

impl PubScreen {
    #[track_caller]
    fn adopt(i: *mut IntuitionIFace, scr: *mut sys::Screen) -> Self {
        Self { i, scr, _t: crate::track::register("public screen lock", i as usize, scr as usize, release_screen), _p: PhantomData }
    }

    /// LockPubScreen(NULL): the default public screen (usually Workbench).
    #[track_caller]
    pub fn default_screen() -> Result<Self> {
        let i = iface()?;
        // SAFETY: an open interface; NULL is the default public screen.
        let scr = unsafe { ((*i).LockPubScreen)(i, core::ptr::null()) };
        if scr.is_null() { Err(AmigaError::NullPointer) } else { Ok(Self::adopt(i, scr)) }
    }

    /// LockPubScreen(`name`).
    #[track_caller]
    pub fn lock(name: &str) -> Result<Self> {
        let i = iface()?;
        let n = c_copy(name.as_bytes())?;
        // SAFETY: an open interface and a NUL-terminated name.
        let scr = unsafe { ((*i).LockPubScreen)(i, n.as_ptr()) };
        if scr.is_null() { Err(AmigaError::NullPointer) } else { Ok(Self::adopt(i, scr)) }
    }

    pub fn size(&self) -> (i32, i32) {
        (rd::<i16>(self.scr as *const u8, SCR_WIDTH) as i32, rd::<i16>(self.scr as *const u8, SCR_HEIGHT) as i32)
    }

    pub fn title(&self) -> String {
        let p: *const u8 = rd(self.scr as *const u8, SCR_TITLE);
        if p.is_null() {
            return String::new();
        }
        // SAFETY: the locked screen's NUL-terminated title, copied.
        String::from_utf8_lossy(unsafe { core::ffi::CStr::from_ptr(p as *const core::ffi::c_char) }.to_bytes()).into_owned()
    }

    /// The locked screen, for a window opened on it (WA_PubScreen).
    #[cfg_attr(not(feature = "reaction"), allow(dead_code))]
    pub(crate) fn as_ptr(&self) -> *mut sys::Screen {
        self.scr
    }

    /// DisplayBeep on this screen.
    pub fn beep(&self) {
        // SAFETY: our locked screen.
        unsafe { ((*self.i).DisplayBeep)(self.i, self.scr) };
    }
}

impl Drop for PubScreen {
    fn drop(&mut self) {
        // SAFETY: our lock, released once; windows on it borrow the lock,
        // so they are closed already.
        unsafe { ((*self.i).UnlockPubScreen)(self.i, core::ptr::null(), self.scr) };
    }
}

/// How to open a window.
#[derive(Clone, Debug)]
pub struct WindowOptions<'a> {
    pub title: &'a str,
    pub left: i32,
    pub top: i32,
    /// The inside size (without borders).
    pub inner_width: u32,
    pub inner_height: u32,
    /// IDCMP classes to receive (the [`idcmp`] constants).
    pub idcmp: u32,
    pub close_gadget: bool,
    pub drag_bar: bool,
    pub depth_gadget: bool,
    pub size_gadget: bool,
    pub activate: bool,
    /// Receive the right button as MOUSEBUTTONS instead of menus.
    pub rmb_trap: bool,
    /// Simple refresh: covered parts are not kept; the window gets
    /// [`Event::RefreshWindow`] and redraws through [`Window::refresh`].
    pub simple_refresh: bool,
}

impl Default for WindowOptions<'_> {
    fn default() -> Self {
        Self {
            title: "",
            left: 0,
            top: 0,
            inner_width: 200,
            inner_height: 100,
            idcmp: idcmp::CLOSEWINDOW,
            close_gadget: true,
            drag_bar: true,
            depth_gadget: true,
            size_gadget: false,
            activate: true,
            rmb_trap: false,
            simple_refresh: false,
        }
    }
}

/// What happened in a window (copied from an IntuiMessage, which was
/// replied).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    CloseWindow,
    /// IDCMP_VANILLAKEY: the character.
    Key(u8),
    /// IDCMP_RAWKEY: the raw code (bit 7: released) and qualifiers.
    RawKey { code: u16, qualifier: u16 },
    /// IDCMP_MOUSEBUTTONS: the code (SELECTDOWN...) and the position in
    /// the window.
    Button { code: u16, x: i16, y: i16 },
    MouseMove { x: i16, y: i16 },
    NewSize,
    ChangeWindow,
    RefreshWindow,
    Active,
    Inactive,
    Tick,
    /// A menu entry chosen: its id, and whether it is now checked (for
    /// toggles).
    Menu { id: u32, checked: bool },
    Other { class: u32, code: u16 },
}

/// An event with its time stamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Message {
    pub event: Event,
    pub qualifier: u16,
    pub seconds: u32,
    pub micros: u32,
}

/// A window other calls can be tied to: a requester's parent, the window
/// a file requester puts to sleep. Implemented by [`Window`] and by
/// ReAction's window; the pointer is NULL while there is no window (a
/// ReAction window iconified), which the calls take as no parent.
pub trait ParentWindow {
    /// The Intuition window, valid while `self` is borrowed, or NULL.
    fn window_ptr(&self) -> *mut sys::Window;
}

impl ParentWindow for Window<'_> {
    fn window_ptr(&self) -> *mut sys::Window {
        self.w
    }
}

/// An open window; CloseWindow on drop (its own port: Intuition frees
/// what is still queued).
pub struct Window<'s> {
    i: *mut IntuitionIFace,
    w: *mut sys::Window,
    // The attached menu strip (borrowed for 's), cleared before closing.
    menu: core::cell::Cell<*mut sys::Menu>,
    // Further items of a multiple menu selection, delivered next.
    queue: core::cell::RefCell<alloc::collections::VecDeque<Message>>,
    // A menuclass strip the window owns (attached with SetMenuStrip), and
    // a context menu with the hook that shows it; both outlive their
    // attachment (cleared before the window closes, dropped after).
    tree: Option<crate::menu::MenuTree>,
    context: Option<(crate::menu::MenuTree, alloc::boxed::Box<dyn core::any::Any>)>,
    // The window's AppWindow (its port borrowed for 's), removed before
    // the window closes.
    #[cfg(feature = "workbench")]
    appwin: *mut sys::AppWindow,
    #[cfg(feature = "workbench")]
    appwin_t: crate::track::Tracked,
    _t: crate::track::Tracked,
    // WA_Title and SetWindowTitles do not copy: the strings live here.
    _title: Vec<u8>,
    _screen_title: Option<Vec<u8>>,
    _s: PhantomData<&'s PubScreen>,
}

impl<'s> Window<'s> {
    /// OpenWindowTagList on `screen`.
    #[track_caller]
    pub fn open(screen: &'s PubScreen, opt: &WindowOptions<'_>) -> Result<Self> {
        let i = screen.i;
        let title = c_copy(opt.title.as_bytes())?;
        let b = |v: bool| v as u32;
        let tags = [
            TagItem { ti_Tag: WA_PUB_SCREEN, ti_Data: screen.scr as u32 },
            TagItem { ti_Tag: WA_LEFT, ti_Data: opt.left as u32 },
            TagItem { ti_Tag: WA_TOP, ti_Data: opt.top as u32 },
            TagItem { ti_Tag: WA_INNER_WIDTH, ti_Data: opt.inner_width },
            TagItem { ti_Tag: WA_INNER_HEIGHT, ti_Data: opt.inner_height },
            TagItem { ti_Tag: WA_TITLE, ti_Data: title.as_ptr() as u32 },
            TagItem { ti_Tag: WA_IDCMP, ti_Data: opt.idcmp },
            TagItem { ti_Tag: WA_CLOSE_GADGET, ti_Data: b(opt.close_gadget) },
            TagItem { ti_Tag: WA_DRAG_BAR, ti_Data: b(opt.drag_bar) },
            TagItem { ti_Tag: WA_DEPTH_GADGET, ti_Data: b(opt.depth_gadget) },
            TagItem { ti_Tag: WA_SIZE_GADGET, ti_Data: b(opt.size_gadget) },
            TagItem { ti_Tag: WA_ACTIVATE, ti_Data: b(opt.activate) },
            TagItem { ti_Tag: WA_RMB_TRAP, ti_Data: b(opt.rmb_trap) },
            TagItem { ti_Tag: WA_NEW_LOOK_MENUS, ti_Data: 1 },
            TagItem { ti_Tag: if opt.simple_refresh { WA_SIMPLE_REFRESH } else { WA_SMART_REFRESH }, ti_Data: 1 },
            TagItem { ti_Tag: WA_NO_CARE_REFRESH, ti_Data: b(!opt.simple_refresh) },
            TagItem { ti_Tag: WA_MIN_WIDTH, ti_Data: 40 },
            TagItem { ti_Tag: WA_MIN_HEIGHT, ti_Data: 20 },
            TagItem { ti_Tag: WA_MAX_WIDTH, ti_Data: 0xFFFF },
            TagItem { ti_Tag: WA_MAX_HEIGHT, ti_Data: 0xFFFF },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: an open interface; a TAG_DONE-ended list naming a screen
        // we hold locked and a title kept in the window.
        let w = unsafe { ((*i).OpenWindowTagList)(i, core::ptr::null(), tags.as_ptr()) };
        if w.is_null() {
            return Err(AmigaError::AllocationFailed);
        }
        Ok(Self {
            i,
            w,
            menu: core::cell::Cell::new(core::ptr::null_mut()),
            queue: core::cell::RefCell::new(alloc::collections::VecDeque::new()),
            tree: None,
            context: None,
            #[cfg(feature = "workbench")]
            appwin: core::ptr::null_mut(),
            #[cfg(feature = "workbench")]
            appwin_t: crate::track::Tracked::none(),
            _t: crate::track::register("window", i as usize, w as usize, release_window),
            _title: title,
            _screen_title: None,
            _s: PhantomData,
        })
    }

    fn b(&self) -> *const u8 {
        self.w as *const u8
    }

    /// Position on the screen (left, top) and outer size.
    pub fn bounds(&self) -> (i32, i32, i32, i32) {
        (
            rd::<i16>(self.b(), WIN_LEFT) as i32,
            rd::<i16>(self.b(), WIN_TOP) as i32,
            rd::<i16>(self.b(), WIN_WIDTH) as i32,
            rd::<i16>(self.b(), WIN_HEIGHT) as i32,
        )
    }

    /// Border widths: left, top, right, bottom.
    pub fn borders(&self) -> (i32, i32, i32, i32) {
        (
            rd::<i8>(self.b(), WIN_BORDER_LEFT) as i32,
            rd::<i8>(self.b(), WIN_BORDER_TOP) as i32,
            rd::<i8>(self.b(), WIN_BORDER_RIGHT) as i32,
            rd::<i8>(self.b(), WIN_BORDER_BOTTOM) as i32,
        )
    }

    /// The inside, in window coordinates: (x0, y0, x1, y1) inclusive.
    pub fn inner(&self) -> (i32, i32, i32, i32) {
        let (_, _, w, h) = self.bounds();
        let (l, t, r, b) = self.borders();
        (l, t, w - r - 1, h - b - 1)
    }

    /// SetWindowTitles: the window's title (the screen title unchanged).
    pub fn set_title(&mut self, title: &str) -> Result<()> {
        let t = c_copy(title.as_bytes())?;
        // SAFETY: our window; the new title is kept in self (the old one
        // is freed only after Intuition has the new one); -1 leaves the
        // screen title.
        unsafe { ((*self.i).SetWindowTitles)(self.i, self.w, t.as_ptr(), usize::MAX as *const u8) };
        self._title = t;
        Ok(())
    }

    /// ActivateWindow (carried out later).
    pub fn activate(&self) {
        // SAFETY: our open window.
        unsafe { ((*self.i).ActivateWindow)(self.i, self.w) };
    }

    /// WindowToFront (carried out later).
    pub fn to_front(&self) {
        // SAFETY: our open window.
        unsafe { ((*self.i).WindowToFront)(self.i, self.w) };
    }

    /// ChangeWindowBox (carried out later: wait for ChangeWindow).
    pub fn change_box(&self, left: i32, top: i32, width: i32, height: i32) {
        // SAFETY: our open window; Intuition limits the box to the screen.
        unsafe { ((*self.i).ChangeWindowBox)(self.i, self.w, left as i16, top as i16, width as i16, height as i16) };
    }

    /// SetWindowPointerA: the busy pointer (after a short delay), or the
    /// normal one again.
    pub fn set_busy(&self, busy: bool) {
        let tags = [
            TagItem { ti_Tag: WA_BUSY_POINTER, ti_Data: busy as u32 },
            TagItem { ti_Tag: WA_POINTER_DELAY, ti_Data: busy as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: our open window and a TAG_DONE-ended list.
        unsafe { ((*self.i).SetWindowPointerA)(self.i, self.w, tags.as_ptr()) };
    }

    /// AddAppWindowA (feature `workbench`): files dropped on this window
    /// arrive on `port` as [`crate::workbench::AppEvent`]s with `id`. The
    /// window must be on the Workbench screen; the AppWindow is removed
    /// before the window closes (or with [`clear_app_window`](Self::clear_app_window)).
    #[cfg(feature = "workbench")]
    #[track_caller]
    pub fn set_app_window(&mut self, port: &'s crate::workbench::AppPort, id: u32) -> Result<()> {
        self.clear_app_window();
        self.appwin = crate::workbench::add_app_window(self.w, port, id)?;
        self.appwin_t = crate::track::register("app window", self.appwin as usize, 0, release_app_window);
        Ok(())
    }

    /// RemoveAppWindow, if the window has one.
    #[cfg(feature = "workbench")]
    pub fn clear_app_window(&mut self) {
        if !self.appwin.is_null() {
            self.appwin_t = crate::track::Tracked::none();
            crate::workbench::remove_app_window(self.appwin);
            self.appwin = core::ptr::null_mut();
        }
    }

    /// ModifyIDCMP: the classes to receive from now on.
    pub fn set_idcmp(&self, classes: u32) {
        // SAFETY: our open window; its port stays (classes non-zero) or
        // Intuition frees what is queued (zero).
        unsafe { ((*self.i).ModifyIDCMP)(self.i, self.w, classes) };
    }

    fn port(&self) -> *mut sys::MsgPort {
        rd(self.b(), WIN_USERPORT)
    }

    /// The signal of the window's port (0 with no IDCMP classes).
    pub fn signal_mask(&self) -> u32 {
        let p = self.port();
        if p.is_null() {
            return 0;
        }
        1u32 << rd::<u8>(p as *const u8, MP_SIGBIT)
    }

    /// The next event, copied and replied; None when none is queued.
    pub fn next(&self) -> Option<Message> {
        if let Some(m) = self.queue.borrow_mut().pop_front() {
            return Some(m);
        }
        let p = self.port();
        if p.is_null() {
            return None;
        }
        // SAFETY: our window's port: GetMsg takes an IntuiMessage off it;
        // its fields are copied, then it is replied once and not touched
        // again.
        unsafe {
            let m = sys::exec_get_msg(p) as *const u8;
            if m.is_null() {
                return None;
            }
            let class: u32 = rd(m, IM_CLASS);
            let code: u16 = rd(m, IM_CODE);
            let qualifier: u16 = rd(m, IM_QUALIFIER);
            let (x, y): (i16, i16) = (rd(m, IM_MOUSE_X), rd(m, IM_MOUSE_Y));
            let (seconds, micros): (u32, u32) = (rd(m, IM_SECONDS), rd(m, IM_MICROS));
            // Menu choices are read from our attached strip before the
            // reply: each item of a multiple selection, by NextSelect (a
            // GadTools strip) or MM_NEXTSELECT (a menuclass tree: the one
            // IAddress names, the strip or the context menu).
            let mut menu_events = Vec::new();
            let strip = self.menu.get();
            let from: *mut sys::APTR = rd(m, IM_IADDRESS);
            let class_tree = [self.tree.as_ref(), self.context.as_ref().map(|c| &c.0)].into_iter().flatten().find(|t| t.is(from));
            if class == idcmp::MENUPICK && class_tree.is_some() {
                for id in crate::menu::selected_of(self.i, from) {
                    menu_events.push(Event::Menu { id, checked: crate::menu::checked_of(self.i, from, id) });
                }
            } else if class == idcmp::MENUPICK && !strip.is_null() && self.tree.is_none() {
                let mut n = code;
                while n != MENUNULL && menu_events.len() < 64 {
                    let item = ((*self.i).ItemAddress)(self.i, strip, n) as *const u8;
                    if item.is_null() {
                        break;
                    }
                    let id: u32 = rd(item, MI_USERDATA);
                    let flags: u16 = rd(item, MI_FLAGS);
                    menu_events.push(Event::Menu { id, checked: flags & CHECKED != 0 });
                    n = rd(item, MI_NEXT_SELECT);
                }
            }
            sys::exec_reply_msg(m as *mut sys::Message);
            if class == idcmp::MENUPICK {
                let mut it = menu_events.into_iter().map(|event| Message { event, qualifier, seconds, micros });
                let first = it.next();
                self.queue.borrow_mut().extend(it);
                return first.or_else(|| self.next());
            }
            let event = match class {
                idcmp::CLOSEWINDOW => Event::CloseWindow,
                idcmp::VANILLAKEY => Event::Key(code as u8),
                idcmp::RAWKEY => Event::RawKey { code, qualifier },
                idcmp::MOUSEBUTTONS => Event::Button { code, x, y },
                idcmp::MOUSEMOVE => Event::MouseMove { x, y },
                idcmp::NEWSIZE => Event::NewSize,
                idcmp::CHANGEWINDOW => Event::ChangeWindow,
                idcmp::REFRESHWINDOW => Event::RefreshWindow,
                idcmp::ACTIVEWINDOW => Event::Active,
                idcmp::INACTIVEWINDOW => Event::Inactive,
                idcmp::INTUITICKS => Event::Tick,
                _ => Event::Other { class, code },
            };
            Some(Message { event, qualifier, seconds, micros })
        }
    }

    /// Wait for the next event, or one of `others`' signals (then None
    /// and the signals that came).
    pub fn wait(&self, others: u32) -> (Option<Message>, u32) {
        loop {
            if let Some(m) = self.next() {
                return (Some(m), 0);
            }
            let got = crate::signal::wait(self.signal_mask() | others);
            if got & others != 0 {
                return (self.next(), got & others);
            }
        }
    }

    /// The window's RastPort, borrowed while `self` is borrowed; drawing is
    /// limited to the window's outer size (layers clip to what is
    /// visible), and its font is put back when the borrow ends.
    #[cfg(feature = "graphics")]
    pub fn rastport(&mut self) -> Result<crate::graphics::RastPort<'_>> {
        let rp: *mut sys::RastPort = rd(self.b(), WIN_RPORT);
        let (_, _, w, h) = self.bounds();
        // SAFETY: our open window's RastPort, valid while the window is
        // (the result borrows it mutably).
        unsafe { crate::graphics::RastPort::borrow_raw(rp, w, h) }
    }

    /// The window for raw calls, valid while self is.
    pub fn as_ptr(&self) -> *mut sys::Window {
        self.w
    }

    /// SetMenuStrip: attach `strip` (borrowed for the window's life; it is
    /// cleared before the window closes). Add [`idcmp::MENUPICK`] to the
    /// window's classes to receive choices.
    #[cfg(feature = "gadtools")]
    pub fn set_menu(&mut self, strip: &'s MenuStrip) -> Result<()> {
        self.clear_menu();
        // SAFETY: our window and a laid-out menu strip that outlives it
        // ('s).
        let ok = unsafe { ((*self.i).SetMenuStrip)(self.i, self.w, strip.menu) } != 0;
        if !ok {
            return Err(AmigaError::InvalidArgument("SetMenuStrip refused the strip"));
        }
        self.menu.set(strip.menu);
        Ok(())
    }

    /// Build `strip` with menuclass (Intuition V54) and attach it
    /// (SetMenuStrip); the window owns the tree. Picks arrive as
    /// [`Event::Menu`] with the items' ids; add [`idcmp::MENUPICK`] to the
    /// window's classes. A GadTools strip attached before is cleared.
    pub fn set_menu_strip(&mut self, strip: &crate::menu::Strip) -> Result<()> {
        let tree = crate::menu::MenuTree::new(strip)?;
        self.clear_menu();
        let root = tree.as_ptr() as *mut sys::Menu;
        // SAFETY: our window and a menuclass root, which Intuition V54
        // takes wherever a Menu chain goes; the tree is kept in self and
        // cleared before it is disposed.
        let ok = unsafe { ((*self.i).SetMenuStrip)(self.i, self.w, root) } != 0;
        if !ok {
            return Err(AmigaError::InvalidArgument("SetMenuStrip refused the menu tree"));
        }
        self.menu.set(root);
        self.tree = Some(tree);
        Ok(())
    }

    /// A context menu (a strip of one menu, built with menuclass) shown
    /// when the right button is pressed over the window's inside; over the
    /// borders and title bar the window's own menus come up as before. Its
    /// picks arrive as [`Event::Menu`] like the strip's.
    pub fn set_context_menu(&mut self, strip: &crate::menu::Strip) -> Result<()> {
        let tree = crate::menu::MenuTree::new(strip)?;
        self.clear_context_menu();
        let (win, root) = (self.w as usize, tree.as_ptr() as usize);
        let hook = crate::hook::Hook::new(move |_obj, msg| {
            let w = win as *const u8;
            let (mx, my) = (rd::<i16>(w, WIN_MOUSE_X) as i32, rd::<i16>(w, WIN_MOUSE_Y) as i32);
            let (ww, wh) = (rd::<i16>(w, WIN_WIDTH) as i32, rd::<i16>(w, WIN_HEIGHT) as i32);
            let (bl, bt) = (rd::<i8>(w, WIN_BORDER_LEFT) as i32, rd::<i8>(w, WIN_BORDER_TOP) as i32);
            let (br, bb) = (rd::<i8>(w, WIN_BORDER_RIGHT) as i32, rd::<i8>(w, WIN_BORDER_BOTTOM) as i32);
            if !msg.is_null() && mx >= bl && my >= bt && mx < ww - br && my < wh - bb {
                // SAFETY: Intuition's ContextMenuMsg for this query; Menu
                // takes the context menu to show (NULL: the window's own).
                unsafe { core::ptr::write_unaligned((msg as *mut u8).add(CMM_MENU) as *mut usize, root) };
            }
            0
        })?;
        let tags = [TagItem { ti_Tag: WA_CONTEXT_MENU_HOOK, ti_Data: hook.as_ptr() as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        // SAFETY: our open window and a hook kept in self until it is
        // taken off again (clear_context_menu, before the window closes).
        unsafe { ((*self.i).SetWindowAttrsA)(self.i, self.w, tags.as_ptr() as *mut TagItem) };
        self.context = Some((tree, alloc::boxed::Box::new(hook)));
        Ok(())
    }

    /// Take the context menu off (WA_ContextMenuHook NULL), then free it.
    pub fn clear_context_menu(&mut self) {
        if self.context.is_some() {
            let tags = [TagItem { ti_Tag: WA_CONTEXT_MENU_HOOK, ti_Data: 0 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
            // SAFETY: our open window; no hook from now on.
            unsafe { ((*self.i).SetWindowAttrsA)(self.i, self.w, tags.as_ptr() as *mut TagItem) };
            self.context = None;
        }
    }

    fn trees(&self) -> impl Iterator<Item = &crate::menu::MenuTree> {
        [self.tree.as_ref(), self.context.as_ref().map(|c| &c.0)].into_iter().flatten()
    }

    /// The check mark of item `id` of the window's menuclass strip or
    /// context menu; None when neither has it.
    pub fn menu_checked(&self, id: u32) -> Option<bool> {
        self.trees().find_map(|t| t.checked(id))
    }

    /// Grey a menuclass item out, or make it usable. False when no tree
    /// has `id`.
    pub fn set_menu_disabled(&self, id: u32, on: bool) -> bool {
        self.trees().fold(false, |any, t| t.set_disabled(id, on) || any)
    }

    /// Set or clear the check mark of toggle `id`, for a setting changed
    /// by a key as well as by the menu: in the menuclass strip or context
    /// menu (MM_SETSTATE), or in a GadTools strip (ClearMenuStrip, the
    /// item's CHECKED flag, ResetMenuStrip). False when no item has that
    /// id.
    pub fn set_menu_checked(&mut self, id: u32, on: bool) -> bool {
        if self.trees().fold(false, |any, t| t.set_checked(id, on) || any) {
            return true;
        }
        #[cfg(feature = "gadtools")]
        if self.tree.is_none() {
            return self.gadtools_set_checked(id, on);
        }
        false
    }

    #[cfg(feature = "gadtools")]
    fn gadtools_set_checked(&mut self, id: u32, on: bool) -> bool {
        let strip = self.menu.get();
        if strip.is_null() {
            return false;
        }
        // SAFETY: our window and its attached strip: ClearMenuStrip first
        // (no menu is in use while the flags change), then the strip's
        // own items are walked (bounded) and one flag word written, then
        // ResetMenuStrip puts the same strip back.
        unsafe {
            ((*self.i).ClearMenuStrip)(self.i, self.w);
            let mut found = false;
            let mut menu = strip as *mut u8;
            let mut menus = 0;
            while !menu.is_null() && menus < 64 && !found {
                let mut item: *mut u8 = rd(menu, MENU_FIRST_ITEM);
                let mut n = 0;
                while !item.is_null() && n < 256 && !found {
                    for it in [item, rd::<*mut u8>(item, MI_SUB_ITEM)] {
                        let mut sub = it;
                        let mut k = 0;
                        while !sub.is_null() && k < 256 {
                            if rd::<u32>(sub, MI_USERDATA) == id {
                                let f: u16 = rd(sub, MI_FLAGS);
                                let f = if on { f | CHECKED } else { f & !CHECKED };
                                core::ptr::write_unaligned(sub.add(MI_FLAGS) as *mut u16, f);
                                found = true;
                                break;
                            }
                            if sub == item {
                                break;
                            }
                            sub = rd(sub, 0);
                            k += 1;
                        }
                        if found {
                            break;
                        }
                    }
                    item = rd(item, 0);
                    n += 1;
                }
                menu = rd(menu, 0);
                menus += 1;
            }
            ((*self.i).ResetMenuStrip)(self.i, self.w, strip);
            found
        }
    }

    /// ClearMenuStrip (waits if the user is in a menu); a menuclass tree
    /// the window owns is freed after.
    pub fn clear_menu(&mut self) {
        if !self.menu.get().is_null() {
            // SAFETY: our window with a strip attached.
            unsafe { ((*self.i).ClearMenuStrip)(self.i, self.w) };
            self.menu.set(core::ptr::null_mut());
        }
        self.tree = None;
    }

    /// BeginRefresh: redraw what was damaged (after an
    /// [`Event::RefreshWindow`]); EndRefresh when the guard is dropped.
    pub fn refresh(&mut self) -> RefreshGuard<'_, 's> {
        // SAFETY: our open window.
        unsafe { ((*self.i).BeginRefresh)(self.i, self.w) };
        RefreshGuard { win: self }
    }
}

/// A window with a clip region installed; the old one is put back on drop.
#[cfg(feature = "layers")]
pub struct ClipGuard<'w, 's, 'r> {
    win: &'w mut Window<'s>,
    l: *mut sys::LayersIFace,
    layer: *mut sys::Layer,
    old: *mut sys::Region,
    _r: PhantomData<&'r crate::graphics::Region>,
}

#[cfg(feature = "layers")]
impl<'s> Window<'s> {
    /// InstallClipRegion: drawing through the guard's
    /// [`rastport`](ClipGuard::rastport) reaches only `region` (window
    /// coordinates). Not to be held across a refresh.
    pub fn clip<'w, 'r>(&'w mut self, region: &'r crate::graphics::Region) -> Result<ClipGuard<'w, 's, 'r>> {
        let l = crate::layers::layers_iface()?;
        let rp: *const u8 = rd(self.b(), WIN_RPORT);
        // The RastPort's Layer at 0.
        let layer: *mut sys::Layer = rd(rp, 0);
        if layer.is_null() {
            return Err(AmigaError::NullPointer);
        }
        // SAFETY: our window's layer and a region borrowed for the guard's
        // life; the old region is kept to be put back.
        let old = unsafe { ((*l).InstallClipRegion)(l, layer, region.as_ptr() as *mut sys::APTR) };
        Ok(ClipGuard { win: self, l, layer, old, _r: PhantomData })
    }
}

#[cfg(feature = "layers")]
impl ClipGuard<'_, '_, '_> {
    pub fn rastport(&mut self) -> Result<crate::graphics::RastPort<'_>> {
        self.win.rastport()
    }
}

#[cfg(feature = "layers")]
impl Drop for ClipGuard<'_, '_, '_> {
    fn drop(&mut self) {
        // SAFETY: the layer we installed on; the region it had before.
        unsafe { ((*self.l).InstallClipRegion)(self.l, self.layer, self.old as *mut sys::APTR) };
    }
}

/// A window between BeginRefresh and EndRefresh: drawing through
/// [`rastport`](Self::rastport) reaches only the damaged parts.
pub struct RefreshGuard<'w, 's> {
    win: &'w mut Window<'s>,
}

impl RefreshGuard<'_, '_> {
    #[cfg(feature = "graphics")]
    pub fn rastport(&mut self) -> Result<crate::graphics::RastPort<'_>> {
        self.win.rastport()
    }
}

impl Drop for RefreshGuard<'_, '_> {
    fn drop(&mut self) {
        // SAFETY: the window we began refreshing; the damage is done.
        unsafe { ((*self.win.i).EndRefresh)(self.win.i, self.win.w, 1) };
    }
}

impl Drop for Window<'_> {
    fn drop(&mut self) {
        #[cfg(feature = "workbench")]
        self.clear_app_window();
        self.clear_context_menu();
        self.clear_menu();
        // SAFETY: our window on its own port, closed once; Intuition replies
        // what is still queued; no RastPort borrow is left (they borrow
        // self).
        unsafe { ((*self.i).CloseWindow)(self.i, self.w) };
    }
}

/// One line of a menu description.
#[cfg(feature = "gadtools")]
#[derive(Clone, Copy, Debug)]
pub enum MenuEntry<'a> {
    /// A menu (the title in the screen's bar).
    Title(&'a str),
    /// An item of the last title, with an optional Amiga-key shortcut.
    Item { label: &'a str, key: Option<char>, id: u32 },
    /// An item with a check mark that toggles.
    Toggle { label: &'a str, key: Option<char>, id: u32, checked: bool },
    /// A sub-item of the last item.
    Sub { label: &'a str, key: Option<char>, id: u32 },
    /// A separator line.
    Bar,
}

#[cfg(feature = "gadtools")]
crate::iface::iface!(gadtools, sys::GadToolsIFace, IGadTools, b"gadtools.library\0", b"main\0", 50);

/// A menu strip made with GadTools (CreateMenusA, LayoutMenusA for a
/// screen); FreeMenus and FreeVisualInfo on drop. Windows borrow it.
#[cfg(feature = "gadtools")]
pub struct MenuStrip {
    gt: *mut sys::GadToolsIFace,
    menu: *mut sys::Menu,
    vi: sys::APTR,
    // The labels and keys: GadTools keeps pointers to them.
    _strings: Vec<Vec<u8>>,
    _p: PhantomData<*const ()>,
}

#[cfg(feature = "gadtools")]
impl MenuStrip {
    /// Build the strip and lay it out for `screen` (its font and pens).
    pub fn new(screen: &PubScreen, entries: &[MenuEntry<'_>]) -> Result<Self> {
        if entries.is_empty() || !matches!(entries[0], MenuEntry::Title(_)) {
            return Err(AmigaError::InvalidArgument("a menu starts with a Title"));
        }
        let gt = gadtools()?;
        let mut strings: Vec<Vec<u8>> = Vec::new();
        // struct NewMenu: 20 bytes, pack(2): Type u8 at 0, Label 2,
        // CommKey 6, Flags u16 10, MutualExclude 12, UserData 16.
        let mut nm: Vec<[u8; 20]> = Vec::with_capacity(entries.len() + 1);
        let mut put = |kind: u8, label: Option<&str>, key: Option<char>, flags: u16, id: u32| -> Result<()> {
            let mut e = [0u8; 20];
            e[0] = kind;
            let lp = match label {
                Some(s) => {
                    strings.push(c_copy(s.as_bytes())?);
                    strings.last().map(|v| v.as_ptr() as u32).unwrap_or(0)
                }
                None => u32::MAX, // NM_BARLABEL
            };
            e[2..6].copy_from_slice(&lp.to_be_bytes());
            if let Some(k) = key {
                let mut b = [0u8; 4];
                strings.push(c_copy(k.encode_utf8(&mut b).as_bytes())?);
                let kp = strings.last().map(|v| v.as_ptr() as u32).unwrap_or(0);
                e[6..10].copy_from_slice(&kp.to_be_bytes());
            }
            e[10..12].copy_from_slice(&flags.to_be_bytes());
            e[16..20].copy_from_slice(&id.to_be_bytes());
            nm.push(e);
            Ok(())
        };
        for e in entries {
            match *e {
                MenuEntry::Title(t) => put(1, Some(t), None, 0, 0)?,
                MenuEntry::Item { label, key, id } => put(2, Some(label), key, 0, id)?,
                MenuEntry::Toggle { label, key, id, checked } => put(2, Some(label), key, 0x0001 | 0x0008 | if checked { CHECKED } else { 0 }, id)?,
                MenuEntry::Sub { label, key, id } => put(3, Some(label), key, 0, id)?,
                MenuEntry::Bar => put(2, None, None, 0, 0)?,
            }
        }
        nm.push([0u8; 20]); // NM_END
        // SAFETY: an open interface and a NM_END-terminated NewMenu array
        // whose strings live in `strings` (kept with the strip).
        let menu = unsafe { ((*gt).CreateMenusA)(gt, nm.as_ptr() as *const sys::NewMenu, core::ptr::null()) };
        if menu.is_null() {
            return Err(AmigaError::AllocationFailed);
        }
        // SAFETY: the screen is locked (borrowed); GetVisualInfo reads it.
        let vi = unsafe { ((*gt).GetVisualInfoA)(gt, screen.scr, core::ptr::null()) };
        let new_look = [TagItem { ti_Tag: TAG_USER + 0x80043, ti_Data: 1 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        // SAFETY: our menu and the screen's visual info.
        let laid = !vi.is_null() && unsafe { ((*gt).LayoutMenusA)(gt, menu, vi, new_look.as_ptr()) } != 0;
        let s = Self { gt, menu, vi, _strings: strings, _p: PhantomData };
        if !laid {
            return Err(AmigaError::AllocationFailed);
        }
        Ok(s)
    }
}

#[cfg(feature = "gadtools")]
impl Drop for MenuStrip {
    fn drop(&mut self) {
        // SAFETY: our menus (no window has them attached: windows borrow
        // the strip) and visual info, freed once.
        unsafe {
            ((*self.gt).FreeMenus)(self.gt, self.menu);
            if !self.vi.is_null() {
                ((*self.gt).FreeVisualInfo)(self.gt, self.vi);
            }
        }
    }
}

/// struct EasyStruct (28 bytes): StructSize, Flags, Title, TextFormat,
/// GadgetFormat, then V53 Screen and TagList (zero).
fn easy_struct(title: &[u8], text: &[u8], gadgets: &[u8]) -> [u32; 7] {
    [28, 0, title.as_ptr() as u32, text.as_ptr() as u32, gadgets.as_ptr() as u32, 0, 0]
}

/// A text shown as it is through a RawDoFmt format: `%` doubled.
fn literal(s: &str) -> Result<Vec<u8>> {
    let mut v = String::with_capacity(s.len());
    for c in s.chars() {
        if c == '%' {
            v.push('%');
        }
        v.push(c);
    }
    c_copy(v.as_bytes())
}

fn gadget_text(buttons: &[&str]) -> Result<Vec<u8>> {
    if buttons.is_empty() || buttons.iter().any(|b| b.contains('|')) {
        return Err(AmigaError::InvalidArgument("buttons: at least one, none containing '|'"));
    }
    let mut s = String::new();
    for (k, b) in buttons.iter().enumerate() {
        if k > 0 {
            s.push('|');
        }
        s.push_str(b);
    }
    literal(&s)
}

/// Intuition numbers the buttons 1..N-1 from the left and the last 0.
fn button_index(r: i32, n: usize) -> Option<usize> {
    match r {
        0 => Some(n - 1),
        k if k > 0 && (k as usize) < n => Some(k as usize - 1),
        _ => None,
    }
}

/// EasyRequestArgs: show `text` with `buttons` (left to right) and wait;
/// the index of the button chosen. On `window`'s screen, or the default
/// public screen.
pub fn easy_request(window: Option<&dyn ParentWindow>, title: &str, text: &str, buttons: &[&str]) -> Result<usize> {
    let i = iface()?;
    let (t, x, g) = (literal(title)?, literal(text)?, gadget_text(buttons)?);
    let es = easy_struct(&t, &x, &g);
    // SAFETY: an open interface, our window or NULL, an EasyStruct whose
    // strings (no format codes left) live across the call, no IDCMP, no
    // arguments.
    let r = unsafe {
        ((*i).EasyRequestArgs)(i, window.map_or(core::ptr::null_mut(), |w| w.window_ptr()), es.as_ptr() as *const sys::EasyStruct, core::ptr::null_mut(), core::ptr::null_mut())
    };
    button_index(r, buttons.len()).ok_or(AmigaError::InvalidArgument("the requester ended without a button"))
}

/// A requester that does not block: BuildEasyRequestArgs, polled with
/// [`poll`](Self::poll); FreeSysRequest on drop.
pub struct Requester {
    i: *mut IntuitionIFace,
    w: *mut sys::Window,
    n: usize,
    _strings: [Vec<u8>; 3],
    _p: PhantomData<*const ()>,
}

impl Requester {
    pub fn open(title: &str, text: &str, buttons: &[&str]) -> Result<Self> {
        let i = iface()?;
        let strings = [literal(title)?, literal(text)?, gadget_text(buttons)?];
        let es = easy_struct(&strings[0], &strings[1], &strings[2]);
        // SAFETY: an open interface; an EasyStruct whose strings are kept
        // with the requester; no IDCMP, no arguments.
        let w = unsafe { ((*i).BuildEasyRequestArgs)(i, core::ptr::null_mut(), es.as_ptr() as *const sys::EasyStruct, 0, core::ptr::null_mut()) };
        // BuildEasyRequest returns 0/1 instead of a window when it could
        // not open one (or a button was implied).
        if (w as usize) <= 1 {
            return Err(AmigaError::AllocationFailed);
        }
        Ok(Self { i, w, n: buttons.len(), _strings: strings, _p: PhantomData })
    }

    /// SysReqHandler without waiting: Some(button index) once one was
    /// chosen.
    pub fn poll(&self) -> Option<usize> {
        // SAFETY: our requester window, not freed yet.
        let r = unsafe { ((*self.i).SysReqHandler)(self.i, self.w, core::ptr::null_mut(), 0) };
        if r == -2 { None } else { button_index(r, self.n) }
    }
}

impl Drop for Requester {
    fn drop(&mut self) {
        // SAFETY: our requester, freed once.
        unsafe { ((*self.i).FreeSysRequest)(self.i, self.w) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_and_text() {
        assert_eq!(button_index(1, 3), Some(0));
        assert_eq!(button_index(2, 3), Some(1));
        assert_eq!(button_index(0, 3), Some(2));
        assert_eq!(button_index(0, 1), Some(0));
        assert_eq!(button_index(-1, 3), None);
        assert_eq!(button_index(3, 3), None);
        assert_eq!(literal("50% off").unwrap(), b"50%% off\0");
        assert_eq!(gadget_text(&["Yes", "No"]).unwrap(), b"Yes|No\0");
        assert!(gadget_text(&["a|b"]).is_err());
        assert!(gadget_text(&[]).is_err());
    }
}

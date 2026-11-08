//! Talking to Workbench, safe (feature `workbench`, with `dos`).
//!
//! - [`AppPort`]: a port for Workbench's AppMessages. [`AppPort::next`]
//!   copies one into an [`AppEvent`] (the files dropped or opened become
//!   paths) and replies it before returning; what is still queued is
//!   replied when the port is dropped.
//! - With `intuition`, `Window::set_app_window`: files dropped on a window
//!   (AddAppWindowA; the window removes it before it closes).
//! - [`AppIcon`] (with `icon`): an icon on the Workbench backdrop, opened or
//!   dropped on (AddAppIconA with the default tool icon; RemoveAppIcon,
//!   then FreeDiskObject, on drop).
//! - [`AppMenuItem`]: an item in Workbench's Tools menu (AddAppMenuItemA;
//!   RemoveAppMenuItem on drop).
//!
//! - [`WbStartup`]: a program started from a Workbench icon takes the
//!   start-up message (the tool and project icons as paths, the tool
//!   types with `icon`) and replies it on drop, without Forbid: the
//!   program's code is handed to DOS to free once the process has ended.
//! - [`tool_types`] (with `icon`): an icon's tool types.
//!
//! The objects borrow the port, so it outlives them.

use alloc::string::String;
use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::cstr::c_copy;
use crate::error::{AmigaError, Result};
use crate::port::AmigaMsgPort;
use crate::sys::{self, WorkbenchIFace};
#[cfg(feature = "icon")]
use crate::sys::APTR;

crate::iface::iface!(iface, WorkbenchIFace, IWorkbench, b"workbench.library\0", b"main\0", 50);

/// struct AppMessage (86 bytes, pack(2)) and struct WBArg (8).
const AM_TYPE: usize = 20;
const AM_USER_DATA: usize = 22;
const AM_ID: usize = 26;
const AM_NUM_ARGS: usize = 30;
const AM_ARG_LIST: usize = 34;
const AM_CLASS: usize = 40;
const AM_MOUSE_X: usize = 42;
const AM_MOUSE_Y: usize = 44;
const WBARG_SIZE: usize = 8;
const WBARG_NAME: usize = 4;

const AMTYPE_APPWINDOW: u16 = 7;
const AMTYPE_APPICON: u16 = 8;
const AMTYPE_APPMENUITEM: u16 = 9;
const AMTYPE_APPWINDOWZONE: u16 = 10;

fn rd<T: Copy>(p: *const u8, off: usize) -> T {
    // SAFETY: callers pass a live AppMessage (or WBArg) and a field offset
    // from its header (tests/layout/workbench.c).
    unsafe { core::ptr::read_unaligned(p.add(off) as *const T) }
}

/// A WBArg's lock and name as one path ("" name: the lock's own path).
fn arg_path(a: *const u8) -> String {
    let lock: u32 = rd(a, 0);
    let name: *const u8 = rd(a, WBARG_NAME);
    let dir = if lock == 0 {
        String::new()
    } else {
        // SAFETY: the WBArg's lock, valid while its message is unreplied
        // (callers read before replying).
        unsafe { crate::dos::fs::Lock::path_of(lock) }.unwrap_or_default()
    };
    let name = if name.is_null() {
        String::new()
    } else {
        // SAFETY: the WBArg's NUL-terminated name, copied.
        String::from_utf8_lossy(unsafe { core::ffi::CStr::from_ptr(name as *const core::ffi::c_char) }.to_bytes()).into_owned()
    };
    if name.is_empty() {
        dir
    } else if dir.is_empty() {
        name
    } else {
        crate::dos::add_part(&dir, &name).unwrap_or(name)
    }
}

// struct Process and struct WBStartup (tests/layout/workbench.c).
const NODE_TYPE: usize = 8;
const NT_PROCESS: u8 = 13;
const PR_MSG_PORT: usize = 92;
const PR_CLI: usize = 172;
const PR_FLAGS: usize = 192;
const PRF_FREESEGLIST: u32 = 1;
const GPSLF_SEG: u32 = 1 << 2;
const SM_SEGMENT: usize = 24;
const SM_NUM_ARGS: usize = 28;
const SM_TOOL_WINDOW: usize = 32;
const SM_ARG_LIST: usize = 36;

static STARTUP_TAKEN: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// The message Workbench starts a program with (struct WBStartup): the
/// tool's icon and the project icons it was started with. Replied on
/// drop, which tells Workbench the program has ended.
///
/// The classic Workbench unloads the program's code (sm_Segment) when it
/// gets the reply, while the program is still returning through it; C
/// start-up code replies under Forbid so that it ends first. This crate
/// uses no Forbid. Workbench 4.1 FE needs none: it sends sm_Segment 0 and
/// starts the process with NP_FreeSeglist, so DOS frees the code once the
/// process has ended (QEMU AmigaOne). For a starter that does
/// set sm_Segment, [`take`](Self::take) hands the code to DOS when the
/// process runs on that same segment list: it sets the process's
/// PRF_FREESEGLIST flag and clears sm_Segment.
/// [`code_handed_to_dos`](Self::code_handed_to_dos) says whether DOS
/// frees the code (false: the starter does, after the reply).
pub struct WbStartup {
    msg: *mut u8,
    handed: bool,
}

impl WbStartup {
    /// The start-up message, when the program was started from Workbench:
    /// None from a Shell (or for a task that is not a process, or when
    /// called a second time). Call it first thing in a program's entry
    /// code: from Workbench it waits for the message on the process port.
    pub fn take() -> Option<Self> {
        let proc = crate::signal::current_task() as *mut u8;
        if rd::<u8>(proc, NODE_TYPE) != NT_PROCESS || rd::<u32>(proc, PR_CLI) != 0 {
            return None;
        }
        if STARTUP_TAKEN.swap(true, core::sync::atomic::Ordering::AcqRel) {
            return None;
        }
        // SAFETY: our own process's message port (a process without a CLI
        // is sent its start-up message there); the message is ours until
        // we reply it.
        let msg = unsafe {
            let port = proc.add(PR_MSG_PORT) as *mut sys::MsgPort;
            sys::exec_wait_port(port);
            sys::exec_get_msg(port)
        } as *mut u8;
        if msg.is_null() {
            return None;
        }
        let mut s = Self { msg, handed: false };
        s.handed = s.hand_code_to_dos(proc);
        Some(s)
    }

    fn hand_code_to_dos(&self, proc: *mut u8) -> bool {
        let seg: u32 = rd(self.msg, SM_SEGMENT);
        let Ok(dos) = crate::dos::any() else { return false };
        // SAFETY: an open interface and our own process.
        let ours = unsafe { ((*dos).GetProcSegList)(dos, proc as *const sys::Process, GPSLF_SEG) };
        let flags: u32 = rd(proc, PR_FLAGS);
        if ours != 0 && flags & PRF_FREESEGLIST != 0 && (seg == 0 || seg == ours) {
            // DOS frees it already and Workbench unloads nothing: what
            // Workbench 4.1 FE does (sm_Segment 0, NP_FreeSeglist; QEMU
            // AmigaOne).
            return true;
        }
        if seg == 0 || ours != seg {
            return false;
        }
        // SAFETY: our own process's pr_Flags (written by this task only
        // while it runs; dos reads it when the process ends) and the
        // unreplied message, ours to change.
        unsafe {
            core::ptr::write_unaligned(proc.add(PR_FLAGS) as *mut u32, flags | PRF_FREESEGLIST);
            core::ptr::write_unaligned(self.msg.add(SM_SEGMENT) as *mut u32, 0);
        }
        true
    }

    /// True when DOS frees the program's code after it ends (see above);
    /// false leaves it to Workbench, after the reply.
    pub fn code_handed_to_dos(&self) -> bool {
        self.handed
    }


    /// The paths of sm_ArgList: the tool itself first, then each project
    /// icon it was started with (a drawer's or disk's path when the icon
    /// has no file name).
    pub fn args(&self) -> Vec<String> {
        let n = rd::<i32>(self.msg, SM_NUM_ARGS).clamp(0, 4096) as usize;
        let list: *const u8 = rd(self.msg, SM_ARG_LIST);
        if list.is_null() {
            return Vec::new();
        }
        // SAFETY: element k of the message's sm_NumArgs WBArgs (the message
        // is unreplied while self lives).
        (0..n).map(|k| arg_path(unsafe { list.add(k * WBARG_SIZE) })).collect()
    }

    /// The project files: [`args`](Self::args) without the tool.
    pub fn files(&self) -> Vec<String> {
        self.args().into_iter().skip(1).collect()
    }

    /// The tool window (the icon's TOOLWINDOW tool type), if any.
    pub fn tool_window(&self) -> Option<String> {
        let p: *const u8 = rd(self.msg, SM_TOOL_WINDOW);
        if p.is_null() {
            return None;
        }
        // SAFETY: Workbench's NUL-terminated string, copied.
        Some(String::from_utf8_lossy(unsafe { core::ffi::CStr::from_ptr(p as *const core::ffi::c_char) }.to_bytes()).into_owned())
    }

    /// The tool icon's tool types (`NAME=value` or `NAME` texts), read
    /// with GetDiskObject on [`args`](Self::args)`[0]`; empty when it has
    /// no icon.
    #[cfg(feature = "icon")]
    pub fn tool_types(&self) -> Vec<String> {
        let Some(tool) = self.args().into_iter().next() else { return Vec::new() };
        tool_types(&tool)
    }
}

impl Drop for WbStartup {
    fn drop(&mut self) {
        // SAFETY: Workbench's start-up message, replied once. Its locks are
        // not used after this (args() copies paths).
        unsafe { sys::exec_reply_msg(self.msg as *mut sys::Message) };
    }
}

/// The tool types of `path`'s icon (`path.info`): GetDiskObject, the
/// do_ToolTypes texts copied, FreeDiskObject. Empty when there is no icon.
#[cfg(feature = "icon")]
pub fn tool_types(path: &str) -> Vec<String> {
    const DO_TOOL_TYPES: usize = 54;
    let Ok(ic) = icon() else { return Vec::new() };
    let Ok(name) = c_copy(path.as_bytes()) else { return Vec::new() };
    // SAFETY: an open interface and a NUL-terminated path.
    let dob = unsafe { ((*ic).GetDiskObject)(ic, name.as_ptr()) } as *const u8;
    if dob.is_null() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut tt: *const *const u8 = rd(dob, DO_TOOL_TYPES);
    let mut k = 0;
    while !tt.is_null() && k < 1024 {
        // SAFETY: the icon's NULL-ended array of NUL-terminated texts,
        // copied before the icon is freed.
        let s = unsafe { *tt };
        if s.is_null() {
            break;
        }
        // SAFETY: as above.
        out.push(String::from_utf8_lossy(unsafe { core::ffi::CStr::from_ptr(s as *const core::ffi::c_char) }.to_bytes()).into_owned());
        // SAFETY: the next element of the same array (stops at its NULL).
        tt = unsafe { tt.add(1) };
        k += 1;
    }
    // SAFETY: the icon from GetDiskObject above, freed once.
    unsafe { ((*ic).FreeDiskObject)(ic, dob as *mut sys::DiskObject) };
    out
}

/// Write `path.info` with these tool types: the file's icon if it has one,
/// otherwise the default icon for what the file is (GetDiskObjectNew),
/// then PutDiskObject.
#[cfg(feature = "icon")]
pub fn set_tool_types(path: &str, types: &[&str]) -> Result<()> {
    const DO_TOOL_TYPES: usize = 54;
    let ic = icon()?;
    let name = c_copy(path.as_bytes())?;
    let texts: Vec<Vec<u8>> = types.iter().map(|t| c_copy(t.as_bytes())).collect::<Result<_>>()?;
    let mut array: Vec<*const u8> = texts.iter().map(|t| t.as_ptr()).collect();
    array.push(core::ptr::null());
    // SAFETY: an open interface and a NUL-terminated path.
    let dob = unsafe { ((*ic).GetDiskObjectNew)(ic, name.as_ptr()) } as *mut u8;
    if dob.is_null() {
        return Err(AmigaError::AllocationFailed);
    }
    // SAFETY: the icon from GetDiskObjectNew: its tool type pointer is
    // swapped for our array (alive until after PutDiskObject) and put back
    // before FreeDiskObject frees the icon's own memory.
    let ok = unsafe {
        let field = dob.add(DO_TOOL_TYPES) as *mut *const *const u8;
        let old = core::ptr::read_unaligned(field);
        core::ptr::write_unaligned(field, array.as_ptr());
        let ok = ((*ic).PutDiskObject)(ic, name.as_ptr(), dob as *mut APTR) != 0;
        core::ptr::write_unaligned(field, old);
        ((*ic).FreeDiskObject)(ic, dob as *mut sys::DiskObject);
        ok
    };
    if ok { Ok(()) } else { Err(AmigaError::DosError(crate::dos::io_err())) }
}

/// Which kind of object a message is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppKind {
    Window,
    Icon,
    MenuItem,
    WindowZone,
    Other(u16),
}

/// An AppMessage, copied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppEvent {
    pub kind: AppKind,
    /// The id the object was added with.
    pub id: u32,
    /// For an AppIcon: what was done with it (0 = opened; the AMCLASSICON
    /// values).
    pub class: u16,
    /// The pointer position (an AppWindow's: in the window).
    pub x: i16,
    pub y: i16,
    /// The files dropped or selected, as paths (a volume: "Name:").
    pub files: Vec<String>,
}

/// A port for AppMessages; replies what is left on drop.
pub struct AppPort {
    port: AmigaMsgPort,
}

impl AppPort {
    pub fn new() -> Result<Self> {
        Ok(Self { port: AmigaMsgPort::new()? })
    }

    pub fn signal_mask(&self) -> u32 {
        self.port.signal_mask()
    }

    /// The next message, copied and replied; None when none is queued.
    pub fn next(&self) -> Option<AppEvent> {
        let m = self.port.get_raw()? as *const u8;
        let kind = match rd::<u16>(m, AM_TYPE) {
            AMTYPE_APPWINDOW => AppKind::Window,
            AMTYPE_APPICON => AppKind::Icon,
            AMTYPE_APPMENUITEM => AppKind::MenuItem,
            AMTYPE_APPWINDOWZONE => AppKind::WindowZone,
            t => AppKind::Other(t),
        };
        let id: u32 = rd(m, AM_ID);
        let _user: u32 = rd(m, AM_USER_DATA);
        let files = app_message_files(m);
        let ev = AppEvent { kind, id, class: rd(m, AM_CLASS), x: rd(m, AM_MOUSE_X), y: rd(m, AM_MOUSE_Y), files };
        // SAFETY: Workbench's message, read above and replied once.
        unsafe { sys::exec_reply_msg(m as *mut sys::Message) };
        Some(ev)
    }

    fn ptr(&self) -> *mut sys::MsgPort {
        self.port.as_ptr()
    }
}

impl Drop for AppPort {
    fn drop(&mut self) {
        // The objects borrow the port and are removed already; reply what
        // Workbench sent before that.
        while let Some(m) = self.port.get_raw() {
            // SAFETY: Workbench's message, replied once, not read.
            unsafe { sys::exec_reply_msg(m) };
        }
    }
}

/// The files an AppMessage carries, as paths (its am_ArgList); the
/// message must be live and unreplied (its locks are read).
pub(crate) fn app_message_files(m: *const u8) -> Vec<String> {
    let n = rd::<i32>(m, AM_NUM_ARGS).clamp(0, 4096) as usize;
    let list: *const u8 = rd(m, AM_ARG_LIST);
    let mut files = Vec::with_capacity(n);
    if !list.is_null() {
        for k in 0..n {
            // SAFETY: element k of the message's am_NumArgs WBArgs.
            let a = unsafe { list.add(k * WBARG_SIZE) };
            files.push(arg_path(a));
        }
    }
    files
}

/// AddAppWindowA for `Window::set_app_window`, which keeps the result
/// and removes it before the window closes.
#[cfg(feature = "intuition")]
pub(crate) fn add_app_window(win: *mut sys::Window, port: &AppPort, id: u32) -> Result<*mut sys::AppWindow> {
    let wb = iface()?;
    // SAFETY: an open interface, the caller's open window and a port that
    // outlives the AppWindow (Window::set_app_window borrows it for the
    // window's life).
    let aw = unsafe { ((*wb).AddAppWindowA)(wb, id, 0, win, port.ptr(), core::ptr::null()) };
    if aw.is_null() { Err(AmigaError::AllocationFailed) } else { Ok(aw) }
}

/// RemoveAppWindow, for the window that added it.
#[cfg(feature = "intuition")]
pub(crate) fn remove_app_window(aw: *mut sys::AppWindow) {
    if let Ok(wb) = iface() {
        // SAFETY: an AppWindow added by add_app_window, removed once, before
        // its window closes.
        unsafe { ((*wb).RemoveAppWindow)(wb, aw) };
    }
}

#[cfg(feature = "icon")]
crate::iface::iface!(icon, sys::IconIFace, IIcon, b"icon.library\0", b"main\0", 50);

/// An icon on the Workbench backdrop; RemoveAppIcon on drop.
#[cfg(feature = "icon")]
/// Remove a registered AppIcon and free its disk object (the panic path;
/// see `crate::track`).
unsafe fn release_app_icon(ai: usize, dobj: usize) {
    if let (Ok(wb), Ok(ic)) = (iface(), icon()) {
        // SAFETY: the registry calls this once, for an AppIcon still on the
        // Workbench.
        unsafe {
            ((*wb).RemoveAppIcon)(wb, ai as *mut sys::AppIcon);
            ((*ic).FreeDiskObject)(ic, dobj as *mut sys::DiskObject);
        }
    }
}

/// Remove a registered Tools menu item.
unsafe fn release_app_menu_item(mi: usize, _: usize) {
    if let Ok(wb) = iface() {
        // SAFETY: as release_app_icon, for an item still in the menu.
        unsafe { ((*wb).RemoveAppMenuItem)(wb, mi as *mut sys::AppMenuItem) };
    }
}


pub struct AppIcon<'a> {
    _t: crate::track::Tracked,
    wb: *mut WorkbenchIFace,
    ic: *mut sys::IconIFace,
    ai: *mut sys::AppIcon,
    dobj: *mut sys::DiskObject,
    _label: Vec<u8>,
    _a: PhantomData<&'a AppPort>,
}

#[cfg(feature = "icon")]
impl<'a> AppIcon<'a> {
    /// AddAppIconA with the default tool icon, labelled `label`; messages
    /// arrive on `port` with `id`.
    #[track_caller]
    pub fn add(port: &'a AppPort, id: u32, label: &str) -> Result<Self> {
        let wb = iface()?;
        let ic = icon()?;
        let label = c_copy(label.as_bytes())?;
        // SAFETY: an open interface; WBTOOL (3).
        let dobj = unsafe { ((*ic).GetDefDiskObject)(ic, 3) };
        if dobj.is_null() {
            return Err(AmigaError::AllocationFailed);
        }
        // SAFETY: an open interface, our port (borrowed for 'a), a label
        // and a disk object kept until the icon is removed.
        let ai = unsafe { ((*wb).AddAppIconA)(wb, id, 0, label.as_ptr(), port.ptr(), 0, dobj, core::ptr::null()) };
        if ai.is_null() {
            // SAFETY: our disk object, not in use.
            unsafe { ((*ic).FreeDiskObject)(ic, dobj) };
            return Err(AmigaError::AllocationFailed);
        }
        Ok(Self { wb, ic, ai, dobj, _t: crate::track::register("app icon", ai as usize, dobj as usize, release_app_icon), _label: label, _a: PhantomData })
    }
}

#[cfg(feature = "icon")]
impl Drop for AppIcon<'_> {
    fn drop(&mut self) {
        // SAFETY: our AppIcon, removed once; then its disk object, which
        // Workbench no longer uses.
        unsafe {
            ((*self.wb).RemoveAppIcon)(self.wb, self.ai);
            ((*self.ic).FreeDiskObject)(self.ic, self.dobj);
        }
    }
}

/// An item in Workbench's Tools menu; RemoveAppMenuItem on drop.
pub struct AppMenuItem<'a> {
    _t: crate::track::Tracked,
    wb: *mut WorkbenchIFace,
    mi: *mut sys::AppMenuItem,
    _label: Vec<u8>,
    _a: PhantomData<&'a AppPort>,
}

impl<'a> AppMenuItem<'a> {
    /// AddAppMenuItemA: `label` in the Tools menu; messages arrive on
    /// `port` with `id` (and the selected icons as files).
    #[track_caller]
    pub fn add(port: &'a AppPort, id: u32, label: &str) -> Result<Self> {
        let wb = iface()?;
        let label = c_copy(label.as_bytes())?;
        // SAFETY: an open interface, our port (borrowed for 'a) and a label
        // kept until the item is removed.
        let mi = unsafe { ((*wb).AddAppMenuItemA)(wb, id, 0, label.as_ptr(), port.ptr(), core::ptr::null()) };
        if mi.is_null() { Err(AmigaError::AllocationFailed) } else { Ok(Self { wb, mi, _t: crate::track::register("app menu item", mi as usize, 0, release_app_menu_item), _label: label, _a: PhantomData }) }
    }
}

impl Drop for AppMenuItem<'_> {
    fn drop(&mut self) {
        // SAFETY: our menu item, removed once.
        unsafe { ((*self.wb).RemoveAppMenuItem)(self.wb, self.mi) };
    }
}

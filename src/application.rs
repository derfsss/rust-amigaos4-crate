//! application.library's "application" interface, safe (feature
//! `application`; interface version 2, as the autodoc requires).
//!
//! [`Application::register`] registers the calling task as an
//! application (RegisterApplicationA); dropping it unregisters
//! (UnregisterApplicationA, after replying whatever is still queued). The
//! library gives each application a message port: [`Application::next`]
//! takes a message off it, copies it into an [`Event`] and replies it
//! before returning, so no library message is ever held by the program.
//! [`Application::send`] sends a [`Request`] to another application (or
//! to all with [`AppId::ALL`]).
//!
//! [`find`], [`find_identifier`] and [`list`] look other applications up.
//! Not wrapped: notifications pop-ups, icon locking, the global
//! application settings, Workbench start-up tags.

use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::{c_char, CStr};

use crate::cstr::c_copy;
use crate::error::{AmigaError, Result};
use crate::sys::{self, ApplicationIFace, MsgPort, TagItem, TAG_DONE, TAG_USER};

crate::iface::iface!(iface, ApplicationIFace, IApplication, b"application.library\0", b"application\0", 53, 2);

// libraries/application.h
const REGAPP_UNIQUE: u32 = TAG_USER + 1;
const REGAPP_URL_IDENTIFIER: u32 = TAG_USER + 23;
const REGAPP_NO_ICON: u32 = TAG_USER + 30;
const REGAPP_HIDDEN: u32 = TAG_USER + 31;
const REGAPP_DESCRIPTION: u32 = TAG_USER + 50;
const UNREGAPP_WAIT_FOR_UNLOCK: u32 = TAG_USER + 1;
const FINDAPP_NAME: u32 = TAG_USER + 1;
const FINDAPP_APP_IDENTIFIER: u32 = TAG_USER + 3;
const APPATTR_PORT: u32 = TAG_USER + 1;
const APPATTR_APP_IDENTIFIER: u32 = TAG_USER + 5;

/// struct ApplicationMsg: an exec Message (20 bytes), senderAppID, type,
/// then a STRPTR for OpenDoc/PrintDoc/CustomMsg (tests/layout/application.c).
const MSG_SENDER: usize = 20;
const MSG_TYPE: usize = 24;
const MSG_STRING: usize = 28;
/// struct MsgPort's mp_SigBit (tests/layout/exec.c).
const MP_SIGBIT: usize = 15;

/// An application's ID (never reused while the system runs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AppId(pub u32);

impl AppId {
    /// Every application (a broadcast).
    pub const ALL: AppId = AppId(0);
}

/// What to say when registering.
#[derive(Clone, Copy, Debug, Default)]
pub struct RegisterOptions<'a> {
    /// A reverse-domain-like suffix making the identifier unique
    /// ("name.example.org").
    pub url_identifier: Option<&'a str>,
    /// Refuse a second instance (the first gets [`Event::Unique`]).
    pub unique: bool,
    /// No icon in a dock.
    pub no_icon: bool,
    pub hidden: bool,
    /// A line describing the application (V53.2).
    pub description: Option<&'a str>,
}

/// A request one application sends another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request<'a> {
    Quit,
    ForceQuit,
    Hide,
    Unhide,
    ToFront,
    OpenPrefs,
    ReloadPrefs,
    NewBlankDoc,
    OpenDoc(&'a str),
    PrintDoc(&'a str),
    /// APPLIBMT_CustomMsg (V53.2): a string between cooperating programs.
    Custom(&'a str),
}

/// What arrived on the application's port (a copy; the message itself
/// was replied).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    AppRegister,
    AppUnregister,
    AppDidChangeIconType,
    AppDidChangeHidden,
    LastAppDocChange,
    Quit,
    ForceQuit,
    Hide,
    Unhide,
    ToFront,
    /// A second instance of a unique application was started.
    Unique,
    OpenPrefs,
    ReloadPrefs,
    NewBlankDoc,
    OpenDoc(String),
    PrintDoc(String),
    BlankerAllow,
    BlankerDisallow,
    ConfirmClose,
    GameModeEntered,
    GameModeLeft,
    Custom(String),
    Other(u32),
}

impl Request<'_> {
    fn code(&self) -> u32 {
        match self {
            Request::Quit => 100,
            Request::ForceQuit => 101,
            Request::Hide => 102,
            Request::Unhide => 103,
            Request::ToFront => 104,
            Request::OpenPrefs => 200,
            Request::ReloadPrefs => 201,
            Request::NewBlankDoc => 300,
            Request::OpenDoc(_) => 301,
            Request::PrintDoc(_) => 302,
            Request::Custom(_) => 9999,
        }
    }

    fn text(&self) -> Option<&str> {
        match self {
            Request::OpenDoc(s) | Request::PrintDoc(s) | Request::Custom(s) => Some(s),
            _ => None,
        }
    }
}

fn owned(p: *const u8) -> String {
    if p.is_null() {
        return String::new();
    }
    // SAFETY: a NUL-terminated string of a message or attribute still held
    // (copied before the reply / before the next call).
    String::from_utf8_lossy(unsafe { CStr::from_ptr(p as *const c_char) }.to_bytes()).into_owned()
}

/// A registered application; unregistered on drop.
pub struct Application {
    i: *mut ApplicationIFace,
    id: u32,
    port: *mut MsgPort,
    _p: core::marker::PhantomData<*const ()>,
}

impl Application {
    /// RegisterApplicationA(`name`, ...).
    pub fn register(name: &str, opt: &RegisterOptions<'_>) -> Result<Self> {
        let i = iface()?;
        let n = c_copy(name.as_bytes())?;
        let url = opt.url_identifier.map(|s| c_copy(s.as_bytes())).transpose()?;
        let desc = opt.description.map(|s| c_copy(s.as_bytes())).transpose()?;
        let mut tags = alloc::vec![
            TagItem { ti_Tag: REGAPP_UNIQUE, ti_Data: opt.unique as u32 },
            TagItem { ti_Tag: REGAPP_NO_ICON, ti_Data: opt.no_icon as u32 },
            TagItem { ti_Tag: REGAPP_HIDDEN, ti_Data: opt.hidden as u32 },
        ];
        if let Some(u) = &url {
            tags.push(TagItem { ti_Tag: REGAPP_URL_IDENTIFIER, ti_Data: u.as_ptr() as u32 });
        }
        if let Some(d) = &desc {
            tags.push(TagItem { ti_Tag: REGAPP_DESCRIPTION, ti_Data: d.as_ptr() as u32 });
        }
        tags.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        // SAFETY: an open (v2) interface, a NUL-terminated name and a
        // TAG_DONE-ended list whose strings live across the call.
        let id = unsafe { ((*i).RegisterApplicationA)(i, n.as_ptr(), tags.as_mut_ptr()) };
        if id == 0 {
            return Err(AmigaError::Exists);
        }
        let mut port: *mut MsgPort = core::ptr::null_mut();
        let get = [
            TagItem { ti_Tag: APPATTR_PORT, ti_Data: &mut port as *mut *mut MsgPort as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: our application; APPATTR_Port writes a MsgPort pointer.
        unsafe { ((*i).GetApplicationAttrsA)(i, id, get.as_ptr() as *mut TagItem) };
        let app = Self { i, id, port, _p: core::marker::PhantomData };
        if port.is_null() {
            return Err(AmigaError::NullPointer);
        }
        Ok(app)
    }

    pub fn id(&self) -> AppId {
        AppId(self.id)
    }

    /// The identifier the library gave (name, counter, URL identifier).
    pub fn identifier(&self) -> String {
        let mut p: *const u8 = core::ptr::null();
        let get = [
            TagItem { ti_Tag: APPATTR_APP_IDENTIFIER, ti_Data: &mut p as *mut *const u8 as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: our application; the string is the library's, copied at
        // once.
        unsafe { ((*self.i).GetApplicationAttrsA)(self.i, self.id, get.as_ptr() as *mut TagItem) };
        owned(p)
    }

    /// The signal the application's port raises when a message arrives.
    pub fn signal_mask(&self) -> u32 {
        // SAFETY: the library's port for our application, alive while we
        // are registered.
        1u32 << unsafe { *((self.port as *const u8).add(MP_SIGBIT)) }
    }

    /// The next message, copied and replied; None when the port is empty.
    pub fn next(&self) -> Option<(AppId, Event)> {
        // SAFETY: our application's port: GetMsg takes a message off it;
        // its fields are read, its string copied, then it is replied once
        // and not touched again.
        unsafe {
            let m = sys::exec_get_msg(self.port) as *mut u8;
            if m.is_null() {
                return None;
            }
            let sender = core::ptr::read(m.add(MSG_SENDER) as *const u32);
            let kind = core::ptr::read(m.add(MSG_TYPE) as *const u32);
            let text = || owned(core::ptr::read(m.add(MSG_STRING) as *const *const u8));
            let ev = match kind {
                1 => Event::AppRegister,
                2 => Event::AppUnregister,
                4 => Event::AppDidChangeIconType,
                5 => Event::AppDidChangeHidden,
                6 => Event::LastAppDocChange,
                100 => Event::Quit,
                101 => Event::ForceQuit,
                102 => Event::Hide,
                103 => Event::Unhide,
                104 => Event::ToFront,
                105 => Event::Unique,
                200 => Event::OpenPrefs,
                201 => Event::ReloadPrefs,
                300 => Event::NewBlankDoc,
                301 => Event::OpenDoc(text()),
                302 => Event::PrintDoc(text()),
                400 => Event::BlankerAllow,
                401 => Event::BlankerDisallow,
                470 => Event::ConfirmClose,
                500 => Event::GameModeEntered,
                501 => Event::GameModeLeft,
                9999 => Event::Custom(text()),
                other => Event::Other(other),
            };
            sys::exec_reply_msg(m as *mut sys::Message);
            Some((AppId(sender), ev))
        }
    }

    /// Wait for the next message (or one of `others`' signals: then None
    /// and those signals are returned in the error-free tuple's second
    /// half).
    pub fn wait(&self, others: u32) -> (Option<(AppId, Event)>, u32) {
        loop {
            if let Some(e) = self.next() {
                return (Some(e), 0);
            }
            let got = crate::signal::wait(self.signal_mask() | others);
            if got & others != 0 {
                return (self.next(), got & others);
            }
        }
    }

    /// SendApplicationMsg from this application to `to`.
    pub fn send(&self, to: AppId, req: Request<'_>) -> Result<()> {
        let text = req.text().map(|s| c_copy(s.as_bytes())).transpose()?;
        // struct ApplicationMsg (+ the string pointer), zeroed: the library
        // fills the exec Message part.
        let mut msg = [0u32; 8];
        msg[MSG_SENDER / 4] = self.id;
        msg[MSG_TYPE / 4] = req.code();
        if let Some(t) = &text {
            msg[MSG_STRING / 4] = t.as_ptr() as u32;
        }
        // SAFETY: an open interface, our application, a message of the size
        // its type needs, and its string, alive across the call (the
        // library copies what it delivers: checked by testprogs/libtest).
        let ok = unsafe { ((*self.i).SendApplicationMsg)(self.i, self.id, to.0, msg.as_mut_ptr() as *mut sys::ApplicationMsg, req.code()) } != 0;
        if ok { Ok(()) } else { Err(AmigaError::NullPointer) }
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        while self.next().is_some() {}
        let tags = [
            TagItem { ti_Tag: UNREGAPP_WAIT_FOR_UNLOCK, ti_Data: 0 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: our registration, ended once; its port's messages were
        // replied above.
        unsafe { ((*self.i).UnregisterApplicationA)(self.i, self.id, tags.as_ptr() as *mut TagItem) };
    }
}

fn find_by(tag: u32, s: &str) -> Result<Option<AppId>> {
    let i = iface()?;
    let n = c_copy(s.as_bytes())?;
    let tags = [TagItem { ti_Tag: tag, ti_Data: n.as_ptr() as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
    // SAFETY: an open interface and a TAG_DONE-ended list with a
    // NUL-terminated string.
    let id = unsafe { ((*i).FindApplicationA)(i, tags.as_ptr() as *mut TagItem) };
    Ok((id != 0).then_some(AppId(id)))
}

/// FindApplicationA by name.
pub fn find(name: &str) -> Result<Option<AppId>> {
    find_by(FINDAPP_NAME, name)
}

/// FindApplicationA by identifier ("name.url").
pub fn find_identifier(identifier: &str) -> Result<Option<AppId>> {
    find_by(FINDAPP_APP_IDENTIFIER, identifier)
}

/// GetApplicationList: every registered application, copied (the list is
/// freed at once).
pub fn list() -> Result<Vec<(AppId, String)>> {
    let i = iface()?;
    // SAFETY: an open interface; the list is walked (ApplicationNode:
    // MinNode, appID at 8, name at 12), copied and freed once.
    unsafe {
        let l = ((*i).GetApplicationList)(i);
        if l.is_null() {
            return Ok(Vec::new());
        }
        let mut v = Vec::new();
        let mut n = sys::exec_get_head(l as *mut sys::List);
        while !n.is_null() && v.len() < 4096 {
            let p = n as *const u8;
            v.push((AppId(core::ptr::read(p.add(8) as *const u32)), owned(core::ptr::read(p.add(12) as *const *const u8))));
            n = sys::exec_get_succ(n);
        }
        ((*i).FreeApplicationList)(i, l);
        Ok(v)
    }
}

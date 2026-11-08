//! console.device, safe (feature `console`, with `device` and `timer`).
//!
//! - [`raw_key_convert`]: RawKeyConvert of one raw key and its qualifiers
//!   through the default keymap (console.device opened as a library,
//!   CONU_LIBRARY, once for the program).
//! - [`Console`]: a console in a window that console.device creates and
//!   closes itself (CONFLAG_CREATE_WINDOW): [`write`](Console::write)
//!   (text and CSI sequences), [`read`](Console::read) with a time limit
//!   (an unanswered read is aborted), [`clear_input`](Console::clear_input),
//!   and attributes read with CD_GETATTRS ([`size`](Console::size),
//!   [`cursor`](Console::cursor)). CloseDevice on drop closes the window.
//! - [`WindowConsole`] (feature `intuition`): a console on a window of the
//!   program's own, borrowed while it lives.
//!
//! Not wrapped: keymaps, palettes, notify hooks, the snip functions.

use alloc::vec::Vec;

use crate::cstr::c_copy;
use crate::devio::Device;
use crate::error::{AmigaError, Result};
use crate::sys::{self, ConsoleIFace, TagItem, TAG_DONE, TAG_USER};
use crate::timer::{AmigaTimer, TimerUnit, TimerVal};

const CONSOLE: &[u8] = b"console.device\0";
const CONU_LIBRARY: u32 = u32::MAX;
const CONFLAG_CREATE_WINDOW: u32 = 0x04;
const CMD_READ: u16 = 2;
const CMD_WRITE: u16 = 3;
const CMD_CLEAR: u16 = 5;
const CD_GETATTRS: u16 = 17;
const CON_DUMMY: u32 = TAG_USER + 0x1_0000;
const CDT_GETROWS: u32 = CON_DUMMY + 102;
const CDT_GETCOLUMNS: u32 = CON_DUMMY + 103;
const CDT_GETYCP: u32 = CON_DUMMY + 104;
const CDT_GETXCP: u32 = CON_DUMMY + 105;
const IOSTDREQ_SIZE: u32 = 48;
// WA_* tags for the window console.device creates.
const WA_LEFT: u32 = TAG_USER + 100;
const WA_TOP: u32 = TAG_USER + 101;
const WA_WIDTH: u32 = TAG_USER + 102;
const WA_HEIGHT: u32 = TAG_USER + 103;
const WA_FLAGS: u32 = TAG_USER + 107;
const WA_TITLE: u32 = TAG_USER + 110;
const WFLG_SIZEGADGET: u32 = 0x1;
const WFLG_DRAGBAR: u32 = 0x2;
const WFLG_DEPTHGADGET: u32 = 0x4;
const WFLG_ACTIVATE: u32 = 0x1000;
/// struct InputEvent: 22 bytes; Class at 4, Code 6, Qualifier 8.
const INPUTEVENT_SIZE: usize = 22;
const IECLASS_RAWKEY: u8 = 1;

/// RawKeyConvert: the bytes raw key `code` gives with `qualifier` (the
/// IEQUALIFIER bits) in the default keymap; dead keys are not combined.
pub fn raw_key_convert(code: u16, qualifier: u16) -> Result<Vec<u8>> {
    static SLOT: crate::iface::DeviceSlot = crate::iface::DeviceSlot::new(CONSOLE, CONU_LIBRARY);
    // SAFETY: reading the program's global (NULL when it has none).
    let global = unsafe { sys::IConsole } as *mut sys::Interface;
    let c = SLOT.get(global).ok_or(AmigaError::Device("console.device could not be opened as a library"))? as *mut ConsoleIFace;
    let mut ev = [0u8; INPUTEVENT_SIZE];
    ev[4] = IECLASS_RAWKEY;
    ev[6..8].copy_from_slice(&code.to_be_bytes());
    ev[8..10].copy_from_slice(&qualifier.to_be_bytes());
    let mut buf = [0u8; 32];
    // SAFETY: an open interface, a RAWKEY InputEvent with no next event and
    // no previous-key address, a buffer of the length given, the default
    // keymap.
    let n = unsafe { ((*c).RawKeyConvert)(c, ev.as_ptr() as *const sys::InputEvent, buf.as_mut_ptr(), buf.len() as i32, core::ptr::null()) };
    if n < 0 {
        return Err(AmigaError::InvalidArgument("RawKeyConvert: buffer overflow"));
    }
    Ok(buf[..n as usize].to_vec())
}

/// Which kind of console unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    /// No character map: what is covered is not redrawn.
    Standard = 0,
    /// A character map: the window redraws and resizes its text.
    CharMap = 1,
    /// A character map with cut and paste.
    SnipMap = 3,
}

/// The window console.device creates.
#[derive(Clone, Debug)]
pub struct WindowSpec<'a> {
    pub title: &'a str,
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
    pub activate: bool,
}

/// A console in a window of its own; CloseDevice (and the window) on
/// drop.
pub struct Console {
    dev: Device,
    win: *mut sys::Window,
    _title: Vec<u8>,
}

impl Console {
    /// OpenDevice with CONFLAG_CREATE_WINDOW: console.device opens the
    /// window from `spec` on the default public screen.
    pub fn open(unit: Unit, spec: &WindowSpec<'_>) -> Result<Self> {
        let title = c_copy(spec.title.as_bytes())?;
        let flags = WFLG_DRAGBAR | WFLG_DEPTHGADGET | WFLG_SIZEGADGET | if spec.activate { WFLG_ACTIVATE } else { 0 };
        let mut tags = [
            TagItem { ti_Tag: WA_LEFT, ti_Data: spec.left as u32 },
            TagItem { ti_Tag: WA_TOP, ti_Data: spec.top as u32 },
            TagItem { ti_Tag: WA_WIDTH, ti_Data: spec.width },
            TagItem { ti_Tag: WA_HEIGHT, ti_Data: spec.height },
            TagItem { ti_Tag: WA_TITLE, ti_Data: title.as_ptr() as u32 },
            TagItem { ti_Tag: WA_FLAGS, ti_Data: flags },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        let len = core::mem::size_of_val(&tags) as u32;
        // SAFETY: the taglist (and the title it names, kept with the
        // console) lives across OpenDevice, an IOStdReq-sized request.
        let dev = unsafe { Device::open_with_data(CONSOLE, unit as u32, CONFLAG_CREATE_WINDOW, IOSTDREQ_SIZE, tags.as_mut_ptr() as *mut u8, len) }?;
        let win = dev.request_ref().actual() as *mut sys::Window;
        if win.is_null() {
            return Err(AmigaError::Device("console.device opened no window"));
        }
        Ok(Self { dev, win, _title: title })
    }

    /// CMD_WRITE: text and CSI (0x9B) or ESC [ sequences; the byte count
    /// written.
    pub fn write(&mut self, bytes: &[u8]) -> Result<usize> {
        // SAFETY: CMD_WRITE only reads its buffer.
        unsafe { self.dev.request().do_io_const(CMD_WRITE, bytes, 0) }.map(|n| n as usize)
    }

    /// CMD_READ into `buf`, waiting up to `timeout_ms`: the bytes read, 0
    /// when nothing came (the read is aborted then).
    pub fn read(&mut self, buf: &mut [u8], timeout_ms: u32) -> Result<usize> {
        let mut timer = AmigaTimer::open(TimerUnit::MicroHz)?;
        let mut req = self.dev.new_request()?;
        let dev_mask = self.dev.signal_mask();
        timer.start(TimerVal::from_millis(timeout_ms));
        let r = {
            let fl = req.send_data(CMD_READ, buf, 0)?;
            loop {
                if fl.is_done() {
                    break fl.wait_actual().map(|n| n as usize);
                }
                if timer.is_done() {
                    // Dropping the InFlight aborts and waits for the read.
                    drop(fl);
                    break Ok(0);
                }
                crate::signal::wait(dev_mask | timer.signal_mask());
            }
        };
        if !timer.is_done() {
            timer.abort();
        } else {
            timer.finish();
        }
        r
    }

    /// CMD_CLEAR: forget input not read yet.
    pub fn clear_input(&mut self) -> Result<()> {
        // SAFETY: CMD_CLEAR takes no data.
        unsafe { self.dev.request().do_io_const(CMD_CLEAR, &[], 0) }.map(|_| ())
    }

    /// CD_GETATTRS for 32-bit values.
    fn attrs<const N: usize>(&mut self, which: [u32; N]) -> Result<[i32; N]> {
        let mut out = [0i32; N];
        let mut tags: Vec<TagItem> = which.iter().zip(out.iter_mut()).map(|(&t, o)| TagItem { ti_Tag: t, ti_Data: o as *mut i32 as u32 }).collect();
        tags.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        // SAFETY: tags whose data are the addresses of 32-bit outputs that
        // live across the call (GETROWS/COLUMNS/XCP/YCP write an int32);
        // the device reads the list itself.
        let bytes = unsafe { core::slice::from_raw_parts(tags.as_ptr() as *const u8, tags.len() * 8) };
        // SAFETY: CD_GETATTRS reads the list, writes only through it.
        unsafe { self.dev.request().do_io_const(CD_GETATTRS, bytes, 0) }?;
        Ok(out)
    }

    /// The text area in characters: (rows, columns).
    pub fn size(&mut self) -> Result<(i32, i32)> {
        let [r, c] = self.attrs([CDT_GETROWS, CDT_GETCOLUMNS])?;
        Ok((r, c))
    }

    /// The cursor: (column, row), from 0.
    pub fn cursor(&mut self) -> Result<(i32, i32)> {
        let [x, y] = self.attrs([CDT_GETXCP, CDT_GETYCP])?;
        Ok((x, y))
    }

    /// The window the console writes in, for raw calls (valid while self
    /// is; console.device owns it, or for [`WindowConsole`] the program).
    pub fn window_ptr(&self) -> *mut sys::Window {
        self.win
    }
}

/// A console in a window of the program's own (OpenDevice with io_Data the
/// window): text written into its inner area, keys typed into it while it
/// is active read with [`read`](Console::read). CloseDevice on drop leaves
/// the window open; the borrow keeps the window open while the console
/// lives. Everything [`Console`] does is reached through Deref.
#[cfg(feature = "intuition")]
pub struct WindowConsole<'w> {
    con: Console,
    _win: core::marker::PhantomData<&'w ()>,
}

/// sizeof(struct Window) with SDK 54.25 (ppc-amigaos-gcc 15.3.0;
/// tests/layout/intuition.c).
#[cfg(feature = "intuition")]
const WINDOW_SIZE: u32 = 136;

#[cfg(feature = "intuition")]
impl<'w> WindowConsole<'w> {
    /// Attach a console unit to `window`.
    pub fn open(unit: Unit, window: &'w crate::intuition::Window<'_>) -> Result<Self> {
        let w = window.as_ptr();
        // SAFETY: a live window, borrowed for the console's whole life;
        // console.device reads the Window structure (io_Length its size).
        let dev = unsafe { Device::open_with_data(CONSOLE, unit as u32, 0, IOSTDREQ_SIZE, w as *mut u8, WINDOW_SIZE) }?;
        Ok(Self { con: Console { dev, win: w, _title: Vec::new() }, _win: core::marker::PhantomData })
    }
}

#[cfg(feature = "intuition")]
impl core::ops::Deref for WindowConsole<'_> {
    type Target = Console;
    fn deref(&self) -> &Console {
        &self.con
    }
}

#[cfg(feature = "intuition")]
impl core::ops::DerefMut for WindowConsole<'_> {
    fn deref_mut(&mut self) -> &mut Console {
        &mut self.con
    }
}

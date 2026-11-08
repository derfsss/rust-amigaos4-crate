//! Joysticks, mice and keyboards through AmigaInput, polled, safe (feature
//! `amigainput`).
//!
//! - [`Context`]: AIN_CreateContext (API version 1, no port, no window:
//!   polling), AIN_DeleteContext on drop. [`Context::devices`] copies every
//!   device the enumeration offers into a [`DeviceInfo`].
//! - [`Device`]: AIN_ObtainDevice, AIN_ReleaseDevice on drop; it borrows
//!   the context, so it is released first. Its axis, button and hat
//!   offsets are queried once; [`Device::read`] copies the values out of
//!   the library's data (AIN_ReadDevice) into a [`State`].
//! - Failures carry AmigaInput's error code ([`AinError`], with
//!   AIN_GetErrorString's text).
//!
//! Not wrapped: event mode (a port and AINDP_EVENT), contexts tied to a
//! window, the device requester (undocumented).

use alloc::string::String;
use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::error::{AmigaError, Result};
use crate::sys::{AIN_IFace, APTR};

crate::iface::iface!(iface, AIN_IFace, IAmigaInput, b"AmigaInput.library\0", b"main\0", 50);

const API_VERSION: u32 = 1;
const AINQ_AXIS_OFFSET: u32 = 12;
const AINQ_BUTTON_OFFSET: u32 = 13;
const AINQ_HAT_OFFSET: u32 = 14;
const AINDP_ACTIVE: u32 = 1;
// AIN_Device (pack 2, 28 bytes): DeviceName 0, Type 4, SubType 8,
// NumButtons 12, NumAxes 16, NumHats 20, DeviceID 24.

/// Query parameters for [`Context::query`] (AINQ_*).
pub mod q {
    pub const NUMAXES: u32 = 1;
    pub const NUMBUTTONS: u32 = 2;
    pub const NUMHATS: u32 = 3;
    pub const POLLED: u32 = 7;
    pub const AXIS_SYMMETRIC: u32 = 8;
    pub const SUBCLASS: u32 = 9;
    pub const DRIVERVERSION: u32 = 10;
    pub const AXIS_CLASS: u32 = 11;
    pub const LAYOUT_CLASS: u32 = 15;
    pub const CONNECTED: u32 = 18;
    pub const UNITS: u32 = 19;
}

/// Device types (AINDT_*).
pub const AINDT_MOUSE: u32 = 1;
pub const AINDT_KEYBOARD: u32 = 2;
pub const AINDT_JOYSTICK: u32 = 3;

/// An AmigaInput error: its code (AINERR_*) and text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AinError {
    pub code: u32,
    pub text: String,
}

/// AIN_GetErrorString for `code`.
pub fn error_string(code: u32) -> String {
    let Ok(a) = iface() else { return String::new() };
    // SAFETY: an open interface; the string is the library's, copied.
    let p = unsafe { ((*a).AIN_GetErrorString)(a, code) } as *const u8;
    if p.is_null() {
        return String::new();
    }
    // SAFETY: a NUL-terminated string of the library's, copied.
    String::from_utf8_lossy(unsafe { core::ffi::CStr::from_ptr(p as *const core::ffi::c_char) }.to_bytes()).into_owned()
}

/// A device, as enumerated (copied).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceInfo {
    pub id: u32,
    pub name: String,
    /// AINDT_MOUSE, AINDT_KEYBOARD, AINDT_JOYSTICK.
    pub kind: u32,
    pub subtype: u32,
    pub buttons: u32,
    pub axes: u32,
    pub hats: u32,
}

/// An AmigaInput context (polling); AIN_DeleteContext on drop.
pub struct Context {
    a: *mut AIN_IFace,
    ctx: APTR,
    _p: PhantomData<*const ()>,
}

impl Context {
    /// AIN_CreateContext(1, no tags).
    pub fn new() -> Result<Self> {
        let a = iface()?;
        // SAFETY: an open interface; no tags.
        let ctx = unsafe { ((*a).AIN_CreateContext)(a, API_VERSION, core::ptr::null_mut()) };
        if ctx.is_null() { Err(AmigaError::AllocationFailed) } else { Ok(Self { a, ctx, _p: PhantomData }) }
    }

    /// AIN_GetError for the last call that failed.
    pub fn last_error(&self) -> AinError {
        // SAFETY: our context.
        let code = unsafe { ((*self.a).AIN_GetError)(self.a, self.ctx) };
        AinError { code, text: error_string(code) }
    }

    fn fail(&self, what: &'static str) -> AmigaError {
        let e = self.last_error();
        if e.code == 0 { AmigaError::Device(what) } else { AmigaError::IoError(e.code as i32) }
    }

    /// AIN_EnumDevices: every device, copied.
    pub fn devices(&self) -> Vec<DeviceInfo> {
        let mut out: Vec<DeviceInfo> = Vec::new();
        let f: extern "C" fn(*const u8, *mut Vec<DeviceInfo>) -> u32 = enum_entry;
        // SAFETY: our context; a callback that the library calls on this
        // task during the call, with our Vec as its user data.
        unsafe { ((*self.a).AIN_EnumDevices)(self.a, self.ctx, f as usize as APTR, &mut out as *mut Vec<DeviceInfo> as APTR) };
        out
    }

    /// AIN_Query for a 32-bit answer (the [`q`] parameters and the
    /// offset/class queries), about device `id`, with `arg` (an axis,
    /// button or hat number where the parameter takes one).
    pub fn query(&self, id: u32, param: u32, arg: u32) -> Result<u32> {
        let mut v: u32 = 0;
        // SAFETY: our context; a 4-byte result buffer of the size given.
        let ok = unsafe { ((*self.a).AIN_Query)(self.a, self.ctx, id as usize as APTR, param, arg, &mut v as *mut u32 as APTR, 4) } != 0;
        if ok { Ok(v) } else { Err(self.fail("AIN_Query failed")) }
    }

    /// AIN_ObtainDevice: device `id`, with its offsets queried.
    pub fn obtain(&self, id: u32) -> Result<Device<'_>> {
        let info = self.devices().into_iter().find(|d| d.id == id).ok_or(AmigaError::InvalidArgument("no such AmigaInput device"))?;
        let offsets = |param: u32, n: u32| -> Result<Vec<u32>> { (0..n.min(256)).map(|k| self.query(id, param, k)).collect() };
        let axes = offsets(AINQ_AXIS_OFFSET, info.axes)?;
        let buttons = offsets(AINQ_BUTTON_OFFSET, info.buttons)?;
        let hats = offsets(AINQ_HAT_OFFSET, info.hats)?;
        // SAFETY: our context and a device id it enumerated.
        let h = unsafe { ((*self.a).AIN_ObtainDevice)(self.a, self.ctx, id as usize as APTR) };
        if h.is_null() {
            return Err(self.fail("AIN_ObtainDevice failed"));
        }
        Ok(Device { ctx: self, h, info, axes, buttons, hats })
    }
}

/// AIN_EnumDevices' callback: copy the device, keep enumerating (FALSE).
extern "C" fn enum_entry(dev: *const u8, out: *mut Vec<DeviceInfo>) -> u32 {
    if dev.is_null() || out.is_null() {
        return 0;
    }
    let rd = |off: usize| -> u32 {
        // SAFETY: a live AIN_Device of DEV_SIZE bytes for this call.
        unsafe { core::ptr::read_unaligned(dev.add(off) as *const u32) }
    };
    let name_p = rd(0) as *const u8;
    let name = if name_p.is_null() {
        String::new()
    } else {
        // SAFETY: the device's NUL-terminated name, copied.
        String::from_utf8_lossy(unsafe { core::ffi::CStr::from_ptr(name_p as *const core::ffi::c_char) }.to_bytes()).into_owned()
    };
    // SAFETY: our Vec (from `devices`), not touched elsewhere during the
    // call.
    let out = unsafe { &mut *out };
    if out.len() < 256 {
        out.push(DeviceInfo { id: rd(24), name, kind: rd(4), subtype: rd(8), buttons: rd(12), axes: rd(16), hats: rd(20) });
    }
    0
}

impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: our context, deleted once; its devices borrow it, so they
        // are released already.
        unsafe { ((*self.a).AIN_DeleteContext)(self.a, self.ctx) };
    }
}

/// A device's values at one moment (axes and hats -32768..32767, digital
/// buttons 0/1, analogue buttons 0..32767).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub axes: Vec<i32>,
    pub buttons: Vec<i32>,
    pub hats: Vec<i32>,
}

/// An obtained device; AIN_ReleaseDevice on drop.
pub struct Device<'c> {
    ctx: &'c Context,
    h: *mut APTR,
    info: DeviceInfo,
    axes: Vec<u32>,
    buttons: Vec<u32>,
    hats: Vec<u32>,
}

impl Device<'_> {
    pub fn info(&self) -> &DeviceInfo {
        &self.info
    }

    /// AIN_ReadDevice: the current values, copied out of the library's
    /// data at the offsets it gave.
    pub fn read(&self) -> Result<State> {
        let mut data: APTR = core::ptr::null_mut();
        // SAFETY: our context and device; the library stores a pointer to
        // its own int32 data in `data`.
        let ok = unsafe { ((*self.ctx.a).AIN_ReadDevice)(self.ctx.a, self.ctx.ctx, self.h, &mut data as *mut APTR as *mut APTR) } != 0;
        if !ok || data.is_null() {
            return Err(self.ctx.fail("AIN_ReadDevice failed"));
        }
        let at = |offs: &[u32]| -> Vec<i32> {
            offs.iter()
                .map(|&o| {
                    // SAFETY: element `o` of the library's int32 array, at
                    // an offset the library itself gave for this device.
                    unsafe { core::ptr::read_volatile((data as *const i32).add(o as usize)) }
                })
                .collect()
        };
        Ok(State { axes: at(&self.axes), buttons: at(&self.buttons), hats: at(&self.hats) })
    }

    /// AINDP_ACTIVE: whether this context receives the device's input.
    pub fn set_active(&self, on: bool) -> Result<()> {
        // SAFETY: our context and device.
        let ok = unsafe { ((*self.ctx.a).AIN_SetDeviceParameter)(self.ctx.a, self.ctx.ctx, self.h, AINDP_ACTIVE, on as u32) } != 0;
        if ok { Ok(()) } else { Err(self.ctx.fail("AIN_SetDeviceParameter failed")) }
    }
}

impl Drop for Device<'_> {
    fn drop(&mut self) {
        // SAFETY: our device of our context, released once.
        unsafe { ((*self.ctx.a).AIN_ReleaseDevice)(self.ctx.a, self.ctx.ctx, self.h) };
    }
}


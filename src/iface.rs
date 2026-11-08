//! How the safe modules reach a library interface other than exec's.
//!
//! The raw wrappers in `sys` call through the program's interface globals
//! (`IUtility`, `IDOS`, ...), which clib4's start-up code or a driver's C
//! glue sets. A safe function cannot rely on that: a program may leave a
//! global unset. So each safe module asks [`get`] for its interface: the
//! program's global when it is set, otherwise one the crate opens itself
//! the first time it is needed and keeps for the rest of the program (one
//! open count on a ROM library; never closed, so it can never be dropped
//! while a call is using it).
//!
//! The program must still define the global (clib4 does; a no_std
//! program's glue declares it, NULL is fine): the crate reads it.

use core::sync::atomic::{AtomicPtr, Ordering};

use crate::sys;

/// One interface: the program's global, or the crate's own.
pub(crate) struct Slot {
    own: AtomicPtr<sys::Interface>,
    library: &'static [u8],
    name: &'static [u8],
    version: u32,
    iface_version: u32,
}

impl Slot {
    pub(crate) const fn new(library: &'static [u8], name: &'static [u8], version: u32, iface_version: u32) -> Self {
        Self { own: AtomicPtr::new(core::ptr::null_mut()), library, name, version, iface_version }
    }

    /// The interface, or None if the library cannot be opened. The
    /// program's global is used only if it is at least the interface
    /// version asked for (a version-1 application interface fails with
    /// the current headers).
    pub(crate) fn get(&self, global: *mut sys::Interface) -> Option<*mut sys::Interface> {
        // SAFETY: a set global is an interface the program obtained; its
        // InterfaceData (offset 0) holds its version.
        if !global.is_null() && unsafe { (*(global as *const sys::InterfaceData)).version } >= self.iface_version {
            return Some(global);
        }
        let own = self.own.load(Ordering::Acquire);
        if !own.is_null() {
            return Some(own);
        }
        // SAFETY: NUL-terminated names (the `iface!` callers pass literals
        // with a NUL); OpenLibrary and GetInterface may be called from any
        // task.
        let iface = unsafe {
            let base = sys::exec_open_library(self.library.as_ptr(), self.version);
            if base.is_null() {
                return None;
            }
            let i = sys::exec_get_interface(base, self.name.as_ptr(), self.iface_version, core::ptr::null());
            if i.is_null() {
                sys::exec_close_library(base);
                return None;
            }
            i
        };
        match self.own.compare_exchange(core::ptr::null_mut(), iface, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => Some(iface),
            Err(other) => {
                // Another task got there first: keep its interface, give
                // this one back.
                // SAFETY: the interface opened just above, used by no one.
                unsafe {
                    // struct Interface starts with its InterfaceData.
                    let base = (*(iface as *mut sys::InterfaceData)).lib_base;
                    sys::exec_drop_interface(iface);
                    sys::exec_close_library(base);
                }
                Some(other)
            }
        }
    }
}

/// `iface!(fn_name, IFaceType, GLOBAL, b"lib.library\0", b"main\0", version)`
/// defines `pub(crate) fn fn_name() -> Result<*mut IFaceType>`; a seventh
/// argument gives the interface version (default 1).
macro_rules! iface {
    ($fname:ident, $ty:ty, $global:ident, $lib:expr, $name:expr, $ver:expr) => {
        $crate::iface::iface!($fname, $ty, $global, $lib, $name, $ver, 1);
    };
    ($fname:ident, $ty:ty, $global:ident, $lib:expr, $name:expr, $ver:expr, $iver:expr) => {
        pub(crate) fn $fname() -> $crate::error::Result<*mut $ty> {
            static SLOT: $crate::iface::Slot = $crate::iface::Slot::new($lib, $name, $ver, $iver);
            // SAFETY: reading the program's interface global (a pointer the
            // program sets once at start-up, or leaves NULL).
            let global = unsafe { $crate::sys::$global } as *mut $crate::sys::Interface;
            SLOT.get(global).map(|i| i as *mut $ty).ok_or($crate::error::AmigaError::NullPointer)
        }
    };
}
pub(crate) use iface;

/// An interface a device gives from its base (timer.device's ITimer,
/// console.device's IConsole): the program's global when set, otherwise
/// the device opened once by the crate (a request of its own, never
/// closed) and its interface kept for the rest of the program.
#[cfg_attr(not(any(feature = "timer", feature = "console")), allow(dead_code))]
pub(crate) struct DeviceSlot {
    own: AtomicPtr<sys::Interface>,
    device: &'static [u8],
    unit: u32,
}

#[cfg_attr(not(any(feature = "timer", feature = "console")), allow(dead_code))]
impl DeviceSlot {
    pub(crate) const fn new(device: &'static [u8], unit: u32) -> Self {
        Self { own: AtomicPtr::new(core::ptr::null_mut()), device, unit }
    }

    pub(crate) fn get(&self, global: *mut sys::Interface) -> Option<*mut sys::Interface> {
        if !global.is_null() {
            return Some(global);
        }
        let own = self.own.load(Ordering::Acquire);
        if !own.is_null() {
            return Some(own);
        }
        // AllocSysObject refuses an IORequest without a reply port; this one only opens the device and never does I/O,
        // so a cleared block of shared memory serves.
        let tags = [
            sys::TagItem { ti_Tag: sys::AVT_TYPE, ti_Data: sys::MEMF_SHARED },
            sys::TagItem { ti_Tag: sys::AVT_CLEAR_WITH_VALUE, ti_Data: 0 },
            sys::TagItem { ti_Tag: sys::TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: a TAG_DONE-ended list; the request only opens the device
        // and stays open with it for good; the device name is
        // NUL-terminated (DeviceSlot users pass literals).
        let iface = unsafe {
            let req = sys::exec_alloc_vec_tag_list(48, tags.as_ptr()) as *mut sys::IORequest;
            if req.is_null() {
                return None;
            }
            if sys::exec_open_device(self.device.as_ptr(), self.unit, req, 0) != 0 {
                sys::exec_free_vec(req as sys::APTR);
                return None;
            }
            // io_Device at offset 20 (tests/layout/exec.c).
            let base = core::ptr::read((req as *const u8).add(20) as *const *mut sys::Library);
            let i = sys::exec_get_interface(base, MAIN.as_ptr(), 1, core::ptr::null());
            if i.is_null() {
                sys::exec_close_device(req);
                sys::exec_free_vec(req as sys::APTR);
                return None;
            }
            i
        };
        match self.own.compare_exchange(core::ptr::null_mut(), iface, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => Some(iface),
            // Another task got there first: keep its interface (ours stays
            // open too: one device open count, never used).
            Err(other) => Some(other),
        }
    }
}

#[cfg_attr(not(any(feature = "timer", feature = "console")), allow(dead_code))]
const MAIN: &[u8] = b"main\0";

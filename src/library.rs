//! Opening libraries and their interfaces, closed on drop.
//!
//! [`Library::open`] is OpenLibrary; [`Library::interface`] is
//! GetInterface, and the [`Interface`] it returns borrows the library, so
//! the interface is dropped (DropInterface) before the library is closed
//! (CloseLibrary). The interface's vtable type `I` comes from
//! `amigaos4::sys` (`ExecIFace`, `UtilityIFace`, ...); the raw wrappers
//! in `sys` use their own globals, so with an `Interface` a program calls
//! through `as_ptr()` (`unsafe`, each call with its autodoc's contract).

use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::error::{AmigaError, Result};
use crate::sys::{self, Interface as RawInterface};

/// struct Library's lib_Version, lib_Revision, lib_IdString offsets
/// (tests/layout/exec.c).
const LIB_VERSION: usize = 20;
const LIB_REVISION: usize = 22;
const LIB_IDSTRING: usize = 24;

fn cstring(s: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(s.len() + 1);
    v.extend_from_slice(s.as_bytes());
    v.push(0);
    v
}

/// Close a registered library (the panic path; see `crate::track`).
unsafe fn release_library(base: usize, _: usize) {
    // SAFETY: the registry calls this once, for a library still open; its
    // interfaces were registered later and are dropped first.
    unsafe { sys::exec_close_library(base as *mut sys::Library) };
}

/// Drop a registered interface.
unsafe fn release_interface(i: usize, _: usize) {
    // SAFETY: as release_library, for an interface still obtained.
    unsafe { sys::exec_drop_interface(i as *mut RawInterface) };
}

/// An open library.
pub struct Library {
    _t: crate::track::Tracked,
    base: *mut sys::Library,
}

// SAFETY: a library base may be used and closed from any task.
unsafe impl Send for Library {}
// SAFETY: &self only reads the base's fixed fields.
unsafe impl Sync for Library {}

impl Library {
    /// OpenLibrary(`name`, `version`): `Err(NullPointer)` when the library
    /// cannot be found or is older than `version`.
    #[track_caller]
    pub fn open(name: &str, version: u32) -> Result<Self> {
        let n = cstring(name);
        // SAFETY: a NUL-terminated name.
        let base = unsafe { sys::exec_open_library(n.as_ptr(), version) };
        if base.is_null() { Err(AmigaError::NullPointer) } else { Ok(Self { base, _t: crate::track::register("library", base as usize, 0, release_library) }) }
    }

    pub fn version(&self) -> u16 {
        // SAFETY: lib_Version of an open library.
        unsafe { core::ptr::read((self.base as *const u8).add(LIB_VERSION) as *const u16) }
    }

    pub fn revision(&self) -> u16 {
        // SAFETY: lib_Revision of an open library.
        unsafe { core::ptr::read((self.base as *const u8).add(LIB_REVISION) as *const u16) }
    }

    /// lib_IdString ("name ver.rev (date)"), if set and UTF-8.
    pub fn id_string(&self) -> Option<&str> {
        // SAFETY: lib_IdString of an open library: null or a NUL-terminated
        // string that lives as long as the library is open.
        unsafe {
            let p = core::ptr::read((self.base as *const u8).add(LIB_IDSTRING) as *const *const core::ffi::c_char);
            if p.is_null() {
                return None;
            }
            core::ffi::CStr::from_ptr(p).to_str().ok()
        }
    }

    /// GetInterface(`name`, `version`) as an `I` vtable (`ExecIFace`,
    /// `UtilityIFace`, ...: the caller names the right one).
    #[track_caller]
    pub fn interface<I>(&self, name: &str, version: u32) -> Result<Interface<'_, I>> {
        let n = cstring(name);
        // SAFETY: an open library and a NUL-terminated name.
        let i = unsafe { sys::exec_get_interface(self.base, n.as_ptr(), version, core::ptr::null()) };
        if i.is_null() {
            Err(AmigaError::NullPointer)
        } else {
            Ok(Interface { iface: i, _t: crate::track::register("interface", i as usize, 0, release_interface), _lib: PhantomData, _i: PhantomData })
        }
    }

    pub fn as_ptr(&self) -> *mut sys::Library {
        self.base
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        // SAFETY: opened once, closed once; interfaces borrow the library,
        // so none is left.
        unsafe { sys::exec_close_library(self.base) };
    }
}

/// An interface of an open [`Library`].
pub struct Interface<'l, I> {
    _t: crate::track::Tracked,
    iface: *mut RawInterface,
    _lib: PhantomData<&'l Library>,
    _i: PhantomData<*mut I>,
}

impl<I> Interface<'_, I> {
    /// The interface as its vtable type, for calls through it.
    pub fn as_ptr(&self) -> *mut I {
        self.iface as *mut I
    }
}

impl<I> Drop for Interface<'_, I> {
    fn drop(&mut self) {
        // SAFETY: obtained once from GetInterface, dropped once.
        unsafe { sys::exec_drop_interface(self.iface) };
    }
}

/// OpenResource(`name`): resources are never closed; None when absent.
pub fn open_resource(name: &str) -> Option<core::ptr::NonNull<core::ffi::c_void>> {
    let n = cstring(name);
    // SAFETY: a NUL-terminated name.
    core::ptr::NonNull::new(unsafe { sys::exec_open_resource(n.as_ptr()) })
}

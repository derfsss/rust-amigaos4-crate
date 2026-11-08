//! exec's debug interface (feature `debug`): what code an address belongs
//! to (ObtainDebugSymbol) and the calling task's return addresses
//! (StackTrace), and a signal when the debug output is written to
//! ([`Debug::watch_output`]; the autodoc names its tags RDONA_*, exec/debug.h
//! SDONA_*).
//!
//! [`Debug::open`] opens exec.library's "debug" interface itself, so a
//! program needs no `IDebug` global. Symbols are copied out
//! (ReleaseDebugSymbol is called before returning).

use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::{c_char, CStr};
use core::marker::PhantomData;

use crate::error::{AmigaError, Result};
use crate::hook::Hook;
use crate::library::{Interface, Library};
use crate::signal::{current_task, Signal};
use crate::sys::{self, DebugIFace, TagItem, APTR};

/// What ObtainDebugSymbol knows about an address.
#[derive(Clone, Debug, Default)]
pub struct Symbol {
    /// DEBUG_SYMBOL_* (4: a loaded module, 5: one with stabs).
    pub kind: u32,
    /// The module (the program's path, a library's name).
    pub module: String,
    /// The address's offset in the module's code (kernel types).
    pub offset: u32,
    /// For loaded modules: which segment, and the offset in it.
    pub segment: u32,
    pub segment_offset: u32,
    pub source_file: Option<String>,
    pub source_line: u32,
    pub function: Option<String>,
}

/// exec's debug interface.
pub struct Debug {
    // Field order: the interface is dropped before the library is closed.
    iface: Interface<'static, DebugIFace>,
    _lib: alloc::boxed::Box<Library>,
}

fn string(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    // SAFETY: a NUL-terminated string of a symbol not yet released.
    Some(String::from(unsafe { CStr::from_ptr(p) }.to_str().unwrap_or("?")))
}

impl Debug {
    pub fn open() -> Result<Self> {
        let lib = alloc::boxed::Box::new(Library::open("exec.library", 0)?);
        // SAFETY: the library lives in a Box that `Debug` keeps, declared
        // after the interface, so it outlives the interface; the 'static is
        // that, not a real 'static.
        let lib_ref: &'static Library = unsafe { &*(&*lib as *const Library) };
        let iface = lib_ref.interface::<DebugIFace>("debug", 1)?;
        Ok(Self { iface, _lib: lib })
    }

    /// The symbol for `addr`, if exec knows the code there.
    pub fn symbol(&self, addr: usize) -> Option<Symbol> {
        let i = self.iface.as_ptr();
        // SAFETY: an open debug interface; ObtainDebugSymbol only looks the
        // address up (no tags).
        let s = unsafe { ((*i).ObtainDebugSymbol)(i, addr as APTR, core::ptr::null_mut()) };
        if s.is_null() {
            return None;
        }
        // SAFETY: a struct DebugSymbol (tests/layout/exec.c offsets: Type 4,
        // Name 8, Offset 12, SegmentNumber 16, SegmentOffset 20,
        // SourceFileName 24, SourceLineNumber 28, SourceFunctionName 32),
        // read before it is released.
        let sym = unsafe {
            let w = s as *const u32;
            let r = |i: usize| core::ptr::read(w.add(i));
            Symbol {
                kind: r(1),
                module: string(r(2) as *const c_char).unwrap_or_default(),
                offset: r(3),
                segment: r(4),
                segment_offset: r(5),
                source_file: string(r(6) as *const c_char),
                source_line: r(7),
                function: string(r(8) as *const c_char),
            }
        };
        // SAFETY: the symbol obtained above, released once.
        unsafe { ((*i).ReleaseDebugSymbol)(i, s) };
        Some(sym)
    }

    /// The calling task's return addresses, innermost first, up to `max`.
    ///
    /// exec traces from the task's last saved context (where it last
    /// waited or was switched out), not from this call: the first frames
    /// are exec's and DOS's, the program's own come after them (exec
    /// 54.57: 28 frames, the program's from the 24th).
    pub fn backtrace(&self, max: usize) -> Vec<usize> {
        let Ok(frames) = crate::sync::Mutex::new(Vec::with_capacity(max)) else { return Vec::new() };
        {
            let f = &frames;
            let Ok(hook) = Hook::new(move |_task: APTR, msg: APTR| -> u32 {
                // SAFETY: StackTrace passes a struct StackFrameMsg: State at
                // 4, MemoryAddress at 8 (tests/layout/exec.c).
                let (state, addr) = unsafe {
                    let w = msg as *const u32;
                    (core::ptr::read(w.add(1)), core::ptr::read(w.add(2)) as usize)
                };
                if state != sys::STACK_FRAME_DECODED {
                    return 1;
                }
                let Ok(mut v) = f.lock() else { return 1 };
                v.push(addr);
                (v.len() >= max) as u32
            }) else {
                return Vec::new();
            };
            let i = self.iface.as_ptr();
            // SAFETY: an open debug interface, the calling task (valid while
            // it runs this), and a hook alive across the synchronous trace.
            unsafe { ((*i).StackTrace)(i, current_task(), hook.as_ptr()) };
        }
        frames.into_inner()
    }

    /// Have `signal` set whenever something is written to the debug output
    /// (StartDebugOutputNotify, V53.72; signals are its only way). The watch
    /// ends (EndDebugOutputNotify) when the returned value is dropped, which
    /// the borrows put before the signal is freed and the interface closed.
    ///
    /// # Errors
    ///
    /// `NullPointer` if exec refuses the request.
    pub fn watch_output<'a>(&'a self, signal: &'a Signal) -> Result<OutputWatch<'a>> {
        let tags = [
            TagItem { ti_Tag: sys::SDONA_TASK, ti_Data: current_task() as u32 },
            TagItem { ti_Tag: sys::SDONA_SIGNALMASK, ti_Data: signal.mask() },
            TagItem { ti_Tag: sys::TAG_DONE, ti_Data: 0 },
        ];
        let i = self.iface.as_ptr();
        // SAFETY: an open debug interface and a TAG_DONE-ended list; the task
        // is the caller, which owns `signal` (Signal is not Send).
        let n = unsafe { ((*i).StartDebugOutputNotify)(i, tags.as_ptr() as *mut TagItem) };
        if n.is_null() {
            return Err(AmigaError::NullPointer);
        }
        Ok(OutputWatch { debug: self, notify: n, _signal: PhantomData })
    }
}

/// Debug output being watched: see [`Debug::watch_output`].
pub struct OutputWatch<'a> {
    debug: &'a Debug,
    notify: *mut sys::DebugOutputNotify,
    _signal: PhantomData<&'a Signal>,
}

impl Drop for OutputWatch<'_> {
    fn drop(&mut self) {
        let i = self.debug.iface.as_ptr();
        // SAFETY: the request made in watch_output, ended once, while the
        // interface is still open.
        unsafe { ((*i).EndDebugOutputNotify)(i, self.notify) };
    }
}

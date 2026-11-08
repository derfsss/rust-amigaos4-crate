//! panictest: what a panic leaves behind, with `amigaos4::track`.
//!
//!   panictest [panic|child|check|leak]
//!
//! - `panic` (default): open RAM:panictest.dat for writing, take an
//!   exclusive lock on RAM:panictest.lock, open a window on the default
//!   public screen, allocate a signal and a port, then panic three calls
//!   deep. The tracker should release all of it, write T:panictest.crash,
//!   and return RETURN_FAIL (20) to the Shell.
//! - `child`: a child Process takes the exclusive lock and panics; the
//!   parent's wait must give an error and the lock must be free after.
//! - `check`: takes the exclusive lock and opens the data file for writing
//!   (both fail if a panicked run left them held); prints PASS/FAIL, rc 0/20.
//! - `leak`: forgets a lock and a signal (mem::forget) and returns 0; the
//!   tracker reports both at the end of main and releases them.

#![no_std]
#![no_main]

extern crate alloc;

use core::fmt::{self, Write};
use core::panic::PanicInfo;

use amigaos4::allocator::SharedExecAllocator;
use amigaos4::dos::fs::{File, Lock, LockMode, OpenMode};
use amigaos4::intuition::{PubScreen, Window, WindowOptions};
use amigaos4::port::AmigaMsgPort;
use amigaos4::signal::Signal;
use amigaos4::sys;

#[global_allocator]
static ALLOCATOR: SharedExecAllocator = SharedExecAllocator;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    amigaos4::panic::default_panic_handler(info)
}

struct Out;
impl Write for Out {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut buf = [0u8; 128];
        for chunk in s.as_bytes().chunks(buf.len() - 1) {
            buf[..chunk.len()].copy_from_slice(chunk);
            buf[chunk.len()] = 0;
            // SAFETY: a NUL-terminated buffer on the stack.
            unsafe { sys::dos_put_str(buf.as_ptr() as _) };
        }
        Ok(())
    }
}
macro_rules! say {
    ($($arg:tt)*) => {{ let _ = writeln!(Out, $($arg)*); }};
}

const DATA: &str = "RAM:panictest.dat";
const LOCKED: &str = "RAM:panictest.lock";

#[inline(never)]
fn level3(n: u32) -> u32 {
    if core::hint::black_box(n) > 2 {
        panic!("panictest: deliberate panic at depth {}", n);
    }
    n
}

#[inline(never)]
fn level2(n: u32) -> u32 {
    level3(n + 1) + 1
}

#[inline(never)]
fn level1(n: u32) -> u32 {
    level2(n + 1) + 1
}

fn hold_and_panic() -> i32 {
    let _ = File::write_new(LOCKED, b"x");
    let f = File::open(DATA, OpenMode::Create);
    let l = Lock::new(LOCKED, LockMode::Exclusive);
    let scr = PubScreen::default_screen();
    let w = scr.as_ref().ok().map(|s| Window::open(s, &WindowOptions { title: "panictest", left: 50, top: 50, inner_width: 200, inner_height: 80, activate: false, ..Default::default() }));
    let sig = Signal::alloc();
    let port = AmigaMsgPort::new();
    say!(
        "panictest: holding file {} lock {} window {} signal {} port {}; {} registered",
        f.is_ok(),
        l.is_ok(),
        matches!(w, Some(Ok(_))),
        sig.is_ok(),
        port.is_ok(),
        amigaos4::track::count(amigaos4::signal::current_task())
    );
    let v = level1(core::hint::black_box(1));
    // Not reached.
    drop((f, l, w, sig, port));
    v as i32
}

fn check() -> i32 {
    let l = Lock::new(LOCKED, LockMode::Exclusive);
    let f = File::open(DATA, OpenMode::Create);
    let ok = l.is_ok() && f.is_ok();
    say!("{} check: exclusive lock {:?}, data file {:?}", if ok { "PASS" } else { "FAIL" }, l.as_ref().err(), f.as_ref().err());
    if ok { 0 } else { 20 }
}

fn child() -> i32 {
    let _ = File::write_new(LOCKED, b"x");
    let c = amigaos4::dos::process::spawn("panictest child", || {
        let l = Lock::new(LOCKED, LockMode::Exclusive);
        let held = l.is_ok();
        if core::hint::black_box(held) {
            panic!("panictest child: deliberate panic holding the lock");
        }
        held
    });
    let r = c.map(|c| c.wait());
    let lock_after = Lock::new(LOCKED, LockMode::Exclusive);
    let ok = matches!(r, Ok(Err(_))) && lock_after.is_ok();
    say!("{} child: wait gave {:?}, the lock after it {:?}", if ok { "PASS" } else { "FAIL" }, r.as_ref().map(|x| x.is_err()), lock_after.as_ref().err());
    if ok { 0 } else { 20 }
}

fn leak() -> i32 {
    let _ = File::write_new(LOCKED, b"x");
    if let Ok(l) = Lock::new(LOCKED, LockMode::Exclusive) {
        core::mem::forget(l);
    }
    if let Ok(s) = Signal::alloc() {
        core::mem::forget(s);
    }
    say!("leak: {} registered at the end of main's body", amigaos4::track::count(amigaos4::signal::current_task()));
    0
}

unsafe extern "C" fn mem_handler(_sysbase: *mut core::ffi::c_void, _req: *const sys::MemHandlerData, _data: *mut core::ffi::c_void) -> i32 {
    sys::MEM_DID_NOTHING
}

/// Things the OS calls back into, and system-wide handles: they must all
/// be gone once the program has ended (`panic2`), or have been released by
/// the end-of-main sweep (`leak2`).
fn hold_more(forget: bool) -> i32 {
    let _ = amigaos4::dos::set_var("panictest_var", b"0", amigaos4::dos::VarScope::Global);
    let w = amigaos4::dos::notify::VarWatcher::new("panictest_var");
    // SAFETY: the handler only returns a constant.
    let mh = unsafe { amigaos4::interrupt::MemHandler::add(10, b"panictest\0", mem_handler, core::ptr::null_mut()) };
    let lib = amigaos4::library::Library::open("utility.library", 50);
    let timer = amigaos4::timer::AmigaTimer::open(amigaos4::timer::TimerUnit::MicroHz);
    let bm = amigaos4::graphics::BitMap::new(64, 64, 8, &amigaos4::graphics::BitMapOptions::default());
    let font = amigaos4::graphics::Font::open("topaz.font", 8);
    say!(
        "panictest {}: watcher {} memhandler {} library {} timer {} bitmap {} font {}; {} registered",
        if forget { "leak2" } else { "panic2" },
        w.is_ok(),
        mh.is_ok(),
        lib.is_ok(),
        timer.is_ok(),
        bm.is_ok(),
        font.is_ok(),
        amigaos4::track::count(amigaos4::signal::current_task())
    );
    if forget {
        core::mem::forget((w, mh, lib, timer, bm, font));
        return 0;
    }
    let v = level1(core::hint::black_box(1));
    drop((w, mh, lib, timer, bm, font));
    v as i32
}

/// Make an allocation fail, so exec calls every low-memory handler.
fn lowmem() -> i32 {
    // SAFETY: an allocation that cannot succeed; freed if it does.
    let p = unsafe { sys::exec_alloc_vec(0x7000_0000, 0) };
    if !p.is_null() {
        // SAFETY: the block just allocated.
        unsafe { sys::exec_free_vec(p) };
    }
    say!("PASS lowmem: the low-memory handlers ran and the machine is still here");
    0
}

/// Wait up to 60 s for nothing; a Break (Ctrl-C) must end it at once.
fn wait_for_break() -> i32 {
    say!("wait: waiting up to 60 s; send Ctrl-C");
    let r = amigaos4::timer::Waiter::new().and_then(|mut w| w.wait_ms(0, 60_000));
    let ok = r == Err(amigaos4::error::AmigaError::Cancelled);
    say!("{} wait: ended with {:?}", if ok { "PASS" } else { "FAIL" }, r);
    if ok { 0 } else { 20 }
}

#[no_mangle]
pub extern "C" fn rust_main(args: *const u8, arglen: i32) -> i32 {
    // SAFETY: the Shell's argument string, `arglen` bytes.
    let text = unsafe { core::slice::from_raw_parts(args, arglen.max(0) as usize) };
    let mode = core::str::from_utf8(text).unwrap_or("").split_whitespace().next().unwrap_or("panic");
    match mode {
        "check" => amigaos4::track::main(check),
        "child" => amigaos4::track::main(child),
        "leak" => amigaos4::track::main(leak),
        "panic2" => amigaos4::track::main(|| hold_more(false)),
        "leak2" => amigaos4::track::main(|| hold_more(true)),
        "lowmem" => amigaos4::track::main(lowmem),
        "wait" => amigaos4::track::main(wait_for_break),
        _ => amigaos4::track::main(hold_and_panic),
    }
}

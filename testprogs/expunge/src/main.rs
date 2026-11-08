//! expunge: ask exec to unload a disk-loaded device or library, the
//! classic way and without Forbid.
//!
//!   expunge DEVICE <name> [unit]
//!   expunge LIBRARY <name>
//!
//! It opens the device (unit 0 unless given) or library, calls RemDevice or
//! RemLibrary, and closes it. Being open, its Expunge defers (it sets
//! LIBF_DELEXP); the close is then the last one and runs the deferred
//! expunge, which removes it from exec's list and hands back its segment
//! list. A test for a driver's or library's Close and Expunge: afterwards it
//! must be gone from the list, and opening it again must load it afresh.
//! Returns 0 when the open, Rem* and close were done, 20 otherwise.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{self, Write};
use core::panic::PanicInfo;

use amigaos4::allocator::ExecAllocator;
use amigaos4::device::{IoReq, IOSTDREQ_SIZE};
use amigaos4::port::AmigaMsgPort;
use amigaos4::sys;

#[global_allocator]
static ALLOCATOR: ExecAllocator = ExecAllocator;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    amigaos4::panic::default_panic_handler(info)
}

/// Shell output through dos.library PutStr, in NUL-terminated chunks.
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

/// `name` as a NUL-terminated string.
fn c_name(name: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(name.len() + 1);
    v.extend_from_slice(name.as_bytes());
    v.push(0);
    v
}

fn expunge_device(name: &str, unit: u32) -> Result<(), String> {
    let port = AmigaMsgPort::new().map_err(|e| alloc::format!("CreateMsgPort: {}", e))?;
    let cname = c_name(name);
    // SAFETY: plain exec calls on a request this function owns.
    unsafe {
        let ior = sys::exec_create_iorequest(port.as_ptr(), IOSTDREQ_SIZE);
        if ior.is_null() {
            return Err("CreateIORequest failed".into());
        }
        let err = sys::exec_open_device(cname.as_ptr() as _, unit, ior, 0);
        if err != 0 {
            sys::exec_delete_iorequest(ior);
            return Err(alloc::format!("OpenDevice(\"{}\", {}) returned {}", name, unit, err));
        }
        let dev = IoReq::from_raw(ior).map(|r| r.device()).unwrap_or(core::ptr::null_mut());
        sys::exec_rem_device(dev as *mut sys::Device); // defers: it is open
        sys::exec_close_device(ior); // the last close: the expunge runs
        sys::exec_delete_iorequest(ior);
    }
    Ok(())
}

fn expunge_library(name: &str) -> Result<(), String> {
    let cname = c_name(name);
    // SAFETY: plain exec calls on a library this function opened.
    unsafe {
        let lib = sys::exec_open_library(cname.as_ptr() as _, 0);
        if lib.is_null() {
            return Err(alloc::format!("OpenLibrary(\"{}\", 0) failed", name));
        }
        sys::exec_rem_library(lib); // defers: it is open
        sys::exec_close_library(lib); // the last close: the expunge runs
    }
    Ok(())
}

#[no_mangle]
pub extern "C" fn rust_main(args: *const u8, arglen: i32) -> i32 {
    // SAFETY: the Shell's argument string, `arglen` bytes.
    let text = unsafe { core::slice::from_raw_parts(args, arglen.max(0) as usize) };
    let text = core::str::from_utf8(text).unwrap_or("");
    let words: Vec<&str> = text.split_whitespace().collect();
    let result = match words.as_slice() {
        [kind, name] if kind.eq_ignore_ascii_case("LIBRARY") => expunge_library(name),
        [kind, name] if kind.eq_ignore_ascii_case("DEVICE") => expunge_device(name, 0),
        [kind, name, unit] if kind.eq_ignore_ascii_case("DEVICE") => match unit.parse() {
            Ok(u) => expunge_device(name, u),
            Err(_) => Err(alloc::format!("not a unit number: {}", unit)),
        },
        _ => {
            say!("usage: expunge DEVICE <name> [unit] | expunge LIBRARY <name>");
            return 20;
        }
    };
    match result {
        Ok(()) => {
            say!("expunge: {} opened, removed and closed; if its Close and Expunge are right it is now unloaded", words[1]);
            0
        }
        Err(e) => {
            say!("expunge: FAILED: {}", e);
            20
        }
    }
}

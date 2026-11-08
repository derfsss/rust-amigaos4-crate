//! Reusable panic handler that prints diagnostics to the serial debug output.
//!
//! # Usage
//!
//! In your crate root:
//!
//! ```ignore
//! use core::panic::PanicInfo;
//!
//! #[panic_handler]
//! fn panic(info: &PanicInfo) -> ! {
//!     amigaos4::panic::default_panic_handler(info)
//! }
//! ```

use core::fmt::Write;
use core::panic::PanicInfo;
use crate::fmt::Serial;

/// Default panic handler: [`track::on_panic`](crate::track::on_panic).
///
/// It prints `PANIC at file.rs:42: the message`, the task and a backtrace
/// to the serial debug line (and to `T:<program>.crash` in a DOS
/// Process), releases what the task had registered, and returns to the
/// task's catch point (`track::main`, or the crate's spawned tasks and
/// processes), which then gives RETURN_FAIL or an error from join/wait. A
/// task without a catch point sleeps for good (Wait(0)) instead of
/// spinning.
pub fn default_panic_handler(info: &PanicInfo) -> ! {
    crate::track::on_panic(info)
}

/// The handler before resource tracking: prints the message to the serial
/// debug line, then sleeps for good (Wait(0)). For code that must not touch
/// the registry or jump (it never returns to a catch point).
pub fn print_and_sleep(info: &PanicInfo) -> ! {
    let mut s = Serial;

    let _ = s.write_str("PANIC");

    if let Some(loc) = info.location() {
        let _ = write!(s, " at {}:{}", loc.file(), loc.line());
    }

    let msg = info.message();
    if let Some(literal) = msg.as_str() {
        // Fast path: the panic message is a plain string literal.
        if !literal.is_empty() {
            let _ = write!(s, ": {}", literal);
        }
    } else {
        // The message contains format arguments -- render it.
        let _ = write!(s, ": {}", msg);
    }

    let _ = s.write_str("\n");

    loop {
        // SAFETY: Wait(0) sleeps for good, using no CPU.
        unsafe { crate::sys::exec_wait(0) };
    }
}

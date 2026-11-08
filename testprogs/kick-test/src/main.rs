//! kick-test: rusttest.device, a kickstart module written in Rust.
//!
//! Added to the kickstart (a MODULE line in Kicklayout), it reports on the
//! debug output (serial) as it is loaded:
//!
//! - COLDSTART: exec creates the device during the cold start. The module
//!   prints its Resident, base, seglist, the task running it, the exec
//!   version, free memory, the CPU, and whether dos.library's Resident is
//!   in the kickstart; then it checks the Rust heap.
//! - AFTERDOS: once DOS is up it reports again, opens rusttest.device by
//!   name and reads its unit 0 with DoIO.
//!
//! The last line is `rusttest.device: kickstart load test PASSED` (or
//! FAILED with the reason).

#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicU32, Ordering};

use amigaos4::allocator::SharedExecAllocator;
use amigaos4::port::AmigaMsgPort;
use amigaos4::serial_println;
use amigaos4::sys::{self, IORequest, Message};

#[global_allocator]
// MEMF_SHARED: BeginIO, Open and Close run in the callers' tasks.
static ALLOCATOR: SharedExecAllocator = SharedExecAllocator;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    amigaos4::panic::default_panic_handler(info)
}

const NAME: &str = "rusttest.device";
const DEVNAME_C: &[u8] = amigaos4::amstr!("rusttest.device");

/// What unit 0 returns to CMD_READ.
const GREETING: &[u8] = b"hello from a Rust kickstart module";

/// Counts that the AFTERDOS check reads back.
static COLDSTART_RUNS: AtomicU32 = AtomicU32::new(0);
static OPENS: AtomicU32 = AtomicU32::new(0);

/// kick_glue.c's struct KickInfo.
#[repr(C)]
pub struct KickInfo {
    phase: *const u8,
    task_name: *const u8,
    resident: *const u8,
    rt_flags: u32,
    rt_pri: i32,
    rt_version: u32,
    base: *const u8,
    seglist: u32,
    exec_version: u32,
    exec_revision: u32,
    avail_mem: u32,
    dos_resident_version: u32,
    cpu_model: *const u8,
}

/// A C string from the OS, read up to `max` bytes.
fn c_str(p: *const u8, max: usize) -> &'static str {
    if p.is_null() {
        return "(null)";
    }
    let mut n = 0;
    // SAFETY: the glue passes NUL-terminated OS strings; the scan is capped.
    unsafe {
        while n < max && *p.add(n) != 0 {
            n += 1;
        }
        core::str::from_utf8(core::slice::from_raw_parts(p, n)).unwrap_or("(not UTF-8)")
    }
}

fn flag_names(flags: u32) -> String {
    let mut s = String::new();
    for (bit, name) in [
        (1 << 7, "AUTOINIT"),
        (1 << 5, "NATIVE"),
        (1 << 2, "AFTERDOS"),
        (1 << 1, "SINGLETASK"),
        (1 << 0, "COLDSTART"),
    ] {
        if flags & bit != 0 {
            if !s.is_empty() {
                s.push('|');
            }
            s.push_str(name);
        }
    }
    s
}

fn report(info: &KickInfo) {
    let phase = c_str(info.phase, 16);
    serial_println!("{}: ==== {} init ====", NAME, phase);
    serial_println!(
        "{}: [{}] Resident {:p} flags 0x{:02x} ({}) pri {} version {}",
        NAME, phase, info.resident, info.rt_flags, flag_names(info.rt_flags),
        info.rt_pri, info.rt_version
    );
    if !info.base.is_null() {
        serial_println!("{}: [{}] device base {:p}, seglist 0x{:08x}", NAME, phase, info.base, info.seglist);
    }
    serial_println!(
        "{}: [{}] task \"{}\", exec {}.{}, {} KB free, CPU {}",
        NAME, phase, c_str(info.task_name, 64), info.exec_version, info.exec_revision,
        info.avail_mem / 1024, c_str(info.cpu_model, 64)
    );
    if info.dos_resident_version != 0 {
        serial_println!(
            "{}: [{}] dos.library Resident present, version {}",
            NAME, phase, info.dos_resident_version
        );
    } else {
        serial_println!("{}: [{}] no dos.library Resident found", NAME, phase);
    }
}

/// Called from devInit() during the cold start. Non-zero refuses the device.
#[no_mangle]
pub extern "C" fn rust_kick_coldstart(info: *const KickInfo) -> i32 {
    // SAFETY: the glue passes a filled KickInfo on its stack.
    let info = unsafe { &*info };
    report(info);

    // The Rust heap (SharedExecAllocator) works this early.
    let v: Vec<u32> = (1..=1000).collect();
    let sum: u32 = v.iter().sum();
    if sum != 500_500 {
        serial_println!("{}: [COLDSTART] heap check FAILED (sum {})", NAME, sum);
        return 1;
    }
    serial_println!("{}: [COLDSTART] heap check passed (Vec of {} u32)", NAME, v.len());

    COLDSTART_RUNS.fetch_add(1, Ordering::Relaxed);
    serial_println!("{}: [COLDSTART] device ready, added by exec", NAME);
    0
}

// ---------------------------------------------------------------------------
// The device: unit 0 answers CMD_READ with GREETING
// ---------------------------------------------------------------------------

const CMD_READ: u16 = 2;
const IOERR_OPENFAIL: i8 = -1;
const IOERR_NOCMD: i8 = -3;
const IOF_QUICK: u8 = 1 << 0;
const IOSTDREQ_SIZE: u32 = 48;
const IO_COMMAND: usize = 28;
const IO_FLAGS: usize = 30;
const IO_ERROR: usize = 31;
const IO_ACTUAL: usize = 32;
const IO_LENGTH: usize = 36;
const IO_DATA: usize = 40;

#[no_mangle]
pub extern "C" fn rust_dev_open(_ior: *mut IORequest, unit: u32, _flags: u32) -> i8 {
    if unit != 0 {
        return IOERR_OPENFAIL;
    }
    OPENS.fetch_add(1, Ordering::Relaxed);
    0
}

#[no_mangle]
pub extern "C" fn rust_dev_close(_ior: *mut IORequest) {}

#[no_mangle]
pub extern "C" fn rust_dev_begin_io(ior: *mut IORequest) {
    if ior.is_null() {
        return;
    }
    // SAFETY: an IOStdReq from the caller; offsets are inside IOStdReq.
    unsafe {
        let base = ior as *mut u8;
        let command = *(base.add(IO_COMMAND) as *const u16);
        let mut error = 0i8;
        let mut actual = 0u32;
        if command == CMD_READ {
            let data = *(base.add(IO_DATA) as *const *mut u8);
            let len = (*(base.add(IO_LENGTH) as *const u32) as usize).min(GREETING.len());
            if !data.is_null() {
                core::ptr::copy_nonoverlapping(GREETING.as_ptr(), data, len);
                actual = len as u32;
            }
        } else {
            error = IOERR_NOCMD;
        }
        *(base.add(IO_ERROR) as *mut i8) = error;
        *(base.add(IO_ACTUAL) as *mut u32) = actual;
        if *(base.add(IO_FLAGS) as *const u8) & IOF_QUICK == 0 {
            sys::exec_reply_msg(ior as *mut Message);
        }
    }
}

// ---------------------------------------------------------------------------
// AFTERDOS: report, then use the device
// ---------------------------------------------------------------------------

fn read_through_device() -> Result<String, String> {
    let port = AmigaMsgPort::new().map_err(|e| alloc::format!("CreateMsgPort: {}", e))?;
    // SAFETY: plain exec calls; the request is deleted on every path.
    unsafe {
        let ior = sys::exec_create_iorequest(port.as_ptr(), IOSTDREQ_SIZE);
        if ior.is_null() {
            return Err("CreateIORequest failed".into());
        }
        let err = sys::exec_open_device(DEVNAME_C.as_ptr() as _, 0, ior, 0);
        if err != 0 {
            sys::exec_delete_iorequest(ior);
            return Err(alloc::format!("OpenDevice(\"{}\") returned {}", NAME, err));
        }
        let mut buf = [0u8; 64];
        let base = ior as *mut u8;
        *(base.add(IO_COMMAND) as *mut u16) = CMD_READ;
        *(base.add(IO_DATA) as *mut *mut u8) = buf.as_mut_ptr();
        *(base.add(IO_LENGTH) as *mut u32) = buf.len() as u32;
        let io_err = sys::exec_do_io(ior);
        let actual = *(base.add(IO_ACTUAL) as *const u32) as usize;
        sys::exec_close_device(ior);
        sys::exec_delete_iorequest(ior);

        if io_err != 0 {
            return Err(alloc::format!("DoIO(CMD_READ) error {}", io_err));
        }
        let got = &buf[..actual.min(buf.len())];
        if got != GREETING {
            return Err(alloc::format!("CMD_READ returned {} unexpected bytes", actual));
        }
        Ok(String::from(core::str::from_utf8(got).unwrap_or("?")))
    }
}

/// Called from the AFTERDOS Resident's init.
#[no_mangle]
pub extern "C" fn rust_kick_afterdos(info: *const KickInfo) {
    // SAFETY: the glue passes a filled KickInfo on its stack.
    let info = unsafe { &*info };
    report(info);

    let cold = COLDSTART_RUNS.load(Ordering::Relaxed);
    if cold != 1 {
        serial_println!("{}: kickstart load test FAILED: COLDSTART init ran {} times", NAME, cold);
        return;
    }
    match read_through_device() {
        Ok(text) => {
            serial_println!(
                "{}: [AFTERDOS] OpenDevice + DoIO(CMD_READ) on unit 0: \"{}\" (opens so far: {})",
                NAME, text, OPENS.load(Ordering::Relaxed)
            );
            serial_println!("{}: kickstart load test PASSED", NAME);
        }
        Err(e) => serial_println!("{}: kickstart load test FAILED: {}", NAME, e),
    }
}

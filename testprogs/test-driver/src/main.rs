//! test-driver: target-side tests for the amigaos4 crate in driver mode.
//!
//! A no-C-runtime program (linked with -nostartfiles -nodefaultlibs, like a
//! driver), started from the Shell. It runs each test, prints PASS or FAIL
//! for it to the debug output (serial), then a summary line:
//!
//!     test-driver: N/M tests passed
//!
//! and returns 0 when every test passed, 20 otherwise.
//!
//! The last test opens the example ram.device from RAM_DEVICE_PATH, writes
//! a pattern through CMD_WRITE and reads it back through CMD_READ.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::panic::PanicInfo;

use amigaos4::allocator::ExecAllocator;
use amigaos4::mem::{AmigaPool, AmigaVec, DmaBuffer, DmaDirection, DmaMapping};
use amigaos4::port::AmigaMsgPort;
use amigaos4::serial_println;
use amigaos4::sys;
use amigaos4::tag::TagListBuilder;
use amigaos4::timer::{AmigaTimer, TimerUnit, TimerVal};
use amigaos4::AmigaError;

#[global_allocator]
static ALLOCATOR: ExecAllocator = ExecAllocator;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    amigaos4::panic::default_panic_handler(info)
}

/// Where the test expects the example ram.device.
const RAM_DEVICE_PATH: &[u8] = amigaos4::amstr!("RAM:rusttest/ram.device");

type TestResult = Result<(), String>;

macro_rules! check {
    ($cond:expr, $($msg:tt)*) => {
        if !$cond {
            return Err(alloc::format!($($msg)*));
        }
    };
}

// ---------------------------------------------------------------------------
// Allocator
// ---------------------------------------------------------------------------

fn alloc_vec_string_box() -> TestResult {
    let mut v: Vec<u32> = Vec::new();
    for i in 0..1000 {
        v.push(i);
    }
    check!(v.iter().sum::<u32>() == 499_500, "Vec sum wrong");
    let s = String::from("Rust on AmigaOS 4");
    check!(s.len() == 17, "String length {}", s.len());
    let b = Box::new(0xDEAD_BEEFu32);
    check!(*b == 0xDEAD_BEEF, "Box value wrong");
    Ok(())
}

#[repr(C, align(64))]
struct Aligned64([u8; 64]);

fn alloc_over_aligned() -> TestResult {
    // ExecAllocator over-allocates for alignments above 16.
    let boxes: Vec<Box<Aligned64>> = (0..8).map(|_| Box::new(Aligned64([7; 64]))).collect();
    for b in &boxes {
        let addr = &**b as *const Aligned64 as usize;
        check!(addr % 64 == 0, "address {:#x} not 64-byte aligned", addr);
        check!(b.0.iter().all(|&x| x == 7), "contents wrong");
    }
    Ok(())
}

fn alloc_large() -> TestResult {
    let mut v = vec![0u8; 1 << 20];
    v[0] = 1;
    v[(1 << 20) - 1] = 2;
    check!(v[0] == 1 && v[(1 << 20) - 1] == 2, "1 MiB Vec ends wrong");
    Ok(())
}

// ---------------------------------------------------------------------------
// Memory types
// ---------------------------------------------------------------------------

fn amiga_vec_cleared() -> TestResult {
    let v = AmigaVec::alloc_cleared(4096, sys::MEMF_PRIVATE, 0xA5)
        .map_err(|e| alloc::format!("alloc_cleared: {}", e))?;
    check!(v.len() == 4096, "len {}", v.len());
    check!(v.as_slice().iter().all(|&b| b == 0xA5), "not cleared to 0xA5");
    Ok(())
}

fn amiga_pool() -> TestResult {
    let pool = AmigaPool::new(sys::MEMF_PRIVATE, 8192, 4096)
        .map_err(|e| alloc::format!("CreatePool: {}", e))?;
    let mut ptrs = Vec::new();
    for i in 0..64u8 {
        let p = pool.alloc(256);
        check!(!p.is_null(), "AllocPooled {} returned NULL", i);
        // SAFETY: p points to 256 bytes from the pool.
        unsafe { core::ptr::write_bytes(p, i, 256) };
        ptrs.push(p);
    }
    for (i, &p) in ptrs.iter().enumerate() {
        // SAFETY: as above; nothing else wrote to these blocks.
        let s = unsafe { core::slice::from_raw_parts(p, 256) };
        check!(s.iter().all(|&b| b == i as u8), "block {} overwritten", i);
    }
    // SAFETY: the first block came from this pool's alloc(256).
    unsafe { pool.free(ptrs[0], 256) };
    Ok(())
}

// ---------------------------------------------------------------------------
// memcpy / memmove / memset / memcmp (glue/freestanding_mem.c)
// ---------------------------------------------------------------------------

fn mem_functions() -> TestResult {
    let n = 65536;
    let src: Vec<u8> = (0..n).map(|i| (i * 7 + 3) as u8).collect();
    let mut dst = vec![0u8; n];
    dst.copy_from_slice(&src); // memcpy
    check!(dst == src, "memcpy/memcmp: copy differs"); // memcmp or bcmp

    dst.fill(0x5A); // memset
    check!(dst.iter().all(|&b| b == 0x5A), "memset");

    let mut buf: Vec<u8> = (0..1024u32).map(|i| i as u8).collect();
    buf.copy_within(0..1000, 24); // overlapping, forwards: memmove
    for i in 0..1000 {
        check!(buf[24 + i] == i as u8, "memmove forwards at {}", i);
    }
    let mut buf: Vec<u8> = (0..1024u32).map(|i| i as u8).collect();
    buf.copy_within(24..1024, 0); // overlapping, backwards
    for i in 0..1000 {
        check!(buf[i] == (i + 24) as u8, "memmove backwards at {}", i);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tags and ports
// ---------------------------------------------------------------------------

fn tag_list_builder() -> TestResult {
    let tags = TagListBuilder::new()
        .tag(sys::TAG_USER + 1, 10)
        .tag_if(false, sys::TAG_USER + 2, 20)
        .tag(sys::TAG_USER + 3, 30)
        .build();
    // tag_if(false, ...) keeps its slot as TAG_IGNORE.
    check!(tags.len() == 4, "{} items, expected 4", tags.len());
    check!(tags[0].ti_Data == 10 && tags[2].ti_Data == 30, "wrong data");
    check!(tags[1].ti_Tag == sys::TAG_IGNORE, "skipped tag is not TAG_IGNORE");
    check!(tags[3].ti_Tag == sys::TAG_DONE, "not TAG_DONE terminated");
    Ok(())
}

fn msg_port() -> TestResult {
    let port = AmigaMsgPort::new().map_err(|e| alloc::format!("CreateMsgPort: {}", e))?;
    check!(!port.as_ptr().is_null(), "NULL port");
    let mask = port.signal_mask();
    check!(mask != 0 && mask.count_ones() == 1, "signal mask {:#x}", mask);
    Ok(())
}

// ---------------------------------------------------------------------------
// DMA
// ---------------------------------------------------------------------------

fn dma_to_device() -> TestResult {
    let len = 16384;
    let mut buf = DmaBuffer::alloc(len).map_err(|e| alloc::format!("alloc: {}", e))?;
    for (i, b) in buf.as_mut_slice().iter_mut().enumerate() {
        *b = i as u8;
    }
    let map = buf.map(DmaDirection::ToDevice).map_err(|e| alloc::format!("map: {}", e))?;
    let entries = map.entries();
    check!(!entries.is_empty(), "no DMA entries");
    let total: u32 = entries.iter().map(|e| e.BlockLength).sum();
    check!(total as usize == len, "blocks add up to {} not {}", total, len);
    for e in entries {
        check!(!e.PhysicalAddress.is_null(), "NULL physical address");
    }
    serial_println!(
        "  dma: {} block(s), first phys {:p} len {}",
        entries.len(),
        entries[0].PhysicalAddress,
        entries[0].BlockLength
    );
    drop(map); // EndDMA
    check!(buf.as_slice()[100] == 100, "buffer changed by mapping");
    Ok(())
}

fn dma_from_device_abort() -> TestResult {
    let mut buf = DmaBuffer::alloc(8192).map_err(|e| alloc::format!("alloc: {}", e))?;
    for _ in 0..4 {
        let map = buf.map(DmaDirection::FromDevice).map_err(|e| alloc::format!("map: {}", e))?;
        check!(!map.entries().is_empty(), "no DMA entries");
        map.abort(); // EndDMA with DMAF_NoModify
    }
    Ok(())
}

fn dma_zero_length_refused() -> TestResult {
    let mut byte = 0u8;
    // SAFETY: zero length; StartDMA is never reached.
    let r = unsafe { DmaMapping::new(&mut byte, 0, DmaDirection::ToDevice) };
    check!(matches!(r, Err(AmigaError::DmaFailed)), "zero length was not refused");
    Ok(())
}

// ---------------------------------------------------------------------------
// timer.device
// ---------------------------------------------------------------------------

fn timer_delay() -> TestResult {
    let mut t = AmigaTimer::open(TimerUnit::MicroHz).map_err(|e| alloc::format!("open: {}", e))?;
    let before = t.get_sys_time();
    t.delay(TimerVal::from_millis(200)).map_err(|e| alloc::format!("delay: {}", e))?;
    let after = t.get_sys_time();
    let elapsed = after.as_millis().saturating_sub(before.as_millis());
    serial_println!("  timer: 200 ms delay took {} ms", elapsed);
    check!((150..3000).contains(&elapsed), "elapsed {} ms", elapsed);
    Ok(())
}

// ---------------------------------------------------------------------------
// PowerPC cache and barrier instructions
// ---------------------------------------------------------------------------

fn cache_and_barriers() -> TestResult {
    let mut v = vec![0x33u8; 4096];
    // SAFETY: the range is this Vec's allocation; user-mode instructions.
    unsafe {
        amigaos4::arch::cache_flush(v.as_ptr(), v.len());
        amigaos4::arch::cache_invalidate(v.as_ptr(), v.len());
        amigaos4::arch::sync_barrier();
        amigaos4::arch::lwsync_barrier();
        amigaos4::arch::eieio_barrier();
    }
    v[0] = 1;
    check!(v[0] == 1 && v[4095] == 0x33, "contents changed");
    Ok(())
}

// ---------------------------------------------------------------------------
// The example ram.device, through OpenDevice / DoIO
// ---------------------------------------------------------------------------

const CMD_READ: u16 = 2;
const CMD_WRITE: u16 = 3;
const IOSTDREQ_SIZE: u32 = 48;
const IO_COMMAND: usize = 28;
const IO_ERROR: usize = 31;
const IO_ACTUAL: usize = 32;
const IO_LENGTH: usize = 36;
const IO_DATA: usize = 40;
const IO_OFFSET: usize = 44;

/// Fill in an IOStdReq and run it with DoIO. Returns (io_Error, io_Actual).
unsafe fn do_io(ior: *mut sys::IORequest, cmd: u16, data: *mut u8, len: u32, offset: u32) -> (i8, u32) {
    let base = ior as *mut u8;
    *(base.add(IO_COMMAND) as *mut u16) = cmd;
    *(base.add(IO_DATA) as *mut *mut u8) = data;
    *(base.add(IO_LENGTH) as *mut u32) = len;
    *(base.add(IO_OFFSET) as *mut u32) = offset;
    *(base.add(IO_ACTUAL) as *mut u32) = 0;
    sys::exec_do_io(ior);
    (*(base.add(IO_ERROR) as *const i8), *(base.add(IO_ACTUAL) as *const u32))
}

fn ram_device_roundtrip() -> TestResult {
    let port = AmigaMsgPort::new().map_err(|e| alloc::format!("CreateMsgPort: {}", e))?;
    // SAFETY: plain exec calls; the request is deleted on every path below.
    unsafe {
        let ior = sys::exec_create_iorequest(port.as_ptr(), IOSTDREQ_SIZE);
        check!(!ior.is_null(), "CreateIORequest failed");
        let err = sys::exec_open_device(RAM_DEVICE_PATH.as_ptr() as _, 0, ior, 0);
        if err != 0 {
            sys::exec_delete_iorequest(ior);
            return Err(alloc::format!("OpenDevice returned {}", err));
        }

        let mut out: Vec<u8> = (0..4096u32).map(|i| (i * 13) as u8).collect();
        let mut back = vec![0u8; 4096];
        let (e1, a1) = do_io(ior, CMD_WRITE, out.as_mut_ptr(), 4096, 1024);
        let (e2, a2) = do_io(ior, CMD_READ, back.as_mut_ptr(), 4096, 1024);
        // A read that runs past the 64 KB unit is cut short...
        let mut tail = vec![0u8; 4096];
        let (e3, a3) = do_io(ior, CMD_READ, tail.as_mut_ptr(), 4096, 65536 - 100);
        // ...and one that starts past it fails with IOERR_BADLENGTH (-4).
        let (e4, _) = do_io(ior, CMD_READ, tail.as_mut_ptr(), 16, 65536 + 1);

        sys::exec_close_device(ior);
        sys::exec_delete_iorequest(ior);

        check!(e1 == 0 && a1 == 4096, "CMD_WRITE error {} actual {}", e1, a1);
        check!(e2 == 0 && a2 == 4096, "CMD_READ error {} actual {}", e2, a2);
        check!(back == out, "data read back differs");
        check!(e3 == 0 && a3 == 100, "short read: error {} actual {}, expected 0/100", e3, a3);
        check!(e4 == -4, "read past the end gave error {}, expected -4", e4);
    }
    Ok(())
}

// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn rust_handler_main(_sysbase: *mut u8) -> i32 {
    let tests: &[(&str, fn() -> TestResult)] = &[
        ("alloc_vec_string_box", alloc_vec_string_box),
        ("alloc_over_aligned", alloc_over_aligned),
        ("alloc_large", alloc_large),
        ("amiga_vec_cleared", amiga_vec_cleared),
        ("amiga_pool", amiga_pool),
        ("mem_functions", mem_functions),
        ("tag_list_builder", tag_list_builder),
        ("msg_port", msg_port),
        ("dma_to_device", dma_to_device),
        ("dma_from_device_abort", dma_from_device_abort),
        ("dma_zero_length_refused", dma_zero_length_refused),
        ("timer_delay", timer_delay),
        ("cache_and_barriers", cache_and_barriers),
        ("ram_device_roundtrip", ram_device_roundtrip),
    ];

    serial_println!("test-driver: start ({} tests)", tests.len());
    let mut passed = 0;
    for (name, test) in tests {
        match test() {
            Ok(()) => {
                passed += 1;
                serial_println!("PASS {}", name);
            }
            Err(msg) => serial_println!("FAIL {}: {}", name, msg),
        }
    }
    serial_println!("test-driver: {}/{} tests passed", passed, tests.len());

    if passed == tests.len() { 0 } else { 20 }
}

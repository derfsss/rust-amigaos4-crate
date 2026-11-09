//! Rust for AmigaOS 4.1: SDK bindings, PowerPC helpers, an Exec allocator and
//! safe wrappers, for programs, shared libraries and device drivers.
//!
//! Built for the `powerpc-unknown-amigaos` target of the AmigaLabs Rust fork
//! (<https://github.com/AmigaLabs/rust/tree/amigaos-port>).
//!
//! # Two ways to use it
//!
//! - **Programs** use the default `std` feature. `std` comes from the
//!   AmigaLabs target and runs on clib4.
//! - **Device drivers, libraries and resources** have no C runtime. They turn
//!   off default features, build as `#![no_std]` static libraries, and pick
//!   [`allocator::ExecAllocator`] and [`panic::default_panic_handler`]
//!   themselves:
//!
//! ```toml
//! amigaos4 = { version = "0.1", default-features = false, features = ["expansion"] }
//! ```
//!
//! ```ignore
//! #![no_std]
//! #[global_allocator]
//! static ALLOCATOR: amigaos4::allocator::ExecAllocator = amigaos4::allocator::ExecAllocator;
//!
//! #[panic_handler]
//! fn panic(info: &core::panic::PanicInfo) -> ! {
//!     amigaos4::panic::default_panic_handler(info)
//! }
//! ```
//!
//! # Features
//!
//! Every feature only adds code, so features turned on anywhere in a
//! dependency tree never break a driver build.
//!
//! - `std` (default): implements `std` traits such as `std::error::Error`.
//! - One feature per SDK interface (`dos`, `utility`, `timer`, `expansion`,
//!   `pci`, ...). Exec is always built.
//!
//! # Debug output
//!
//! ```ignore
//! amigaos4::serial_println!("value = {}", 42);
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
// Unsafe code rules: every `unsafe fn` says what its caller must uphold
// (`# Safety`), every `unsafe { }` says why it is sound (`// SAFETY:`), and
// an unsafe fn's body marks each unsafe operation with its own block.
#![warn(clippy::undocumented_unsafe_blocks, clippy::missing_safety_doc, unsafe_op_in_unsafe_fn)]

extern crate alloc;

/// Raw SDK bindings: interface vtables, types, constants and wrapper functions.
pub mod sys;

/// PowerPC instructions: cache maintenance, byte-reversed MMIO, barriers.
#[cfg(target_os = "amigaos")]
pub mod arch;

/// Global allocator for code without a C runtime.
pub mod allocator;
#[cfg(feature = "asl")]
pub mod asl;
#[cfg(feature = "application")]
pub mod application;

pub mod avl;

pub mod cstr;
#[cfg(feature = "debug")]
pub mod debug;
#[cfg(feature = "device")]
pub mod devio;
#[cfg(feature = "device")]
pub mod device;
#[cfg(feature = "dos")]
pub mod dos;
#[cfg(feature = "sana2")]
pub mod sana2;
#[cfg(feature = "sana2")]
pub mod sana2ext;
#[cfg(feature = "bsdsocket")]
pub mod bsdsocket;
pub mod error;
#[cfg(feature = "expansion")]
pub mod expansion;
pub mod fmt;
#[cfg(feature = "graphics")]
pub mod graphics;
pub mod hook;
#[cfg(any(feature = "utility", feature = "pci", feature = "expansion", feature = "dos", feature = "timer", feature = "locale", feature = "application", feature = "prefs_objects", feature = "graphics", feature = "intuition", feature = "gadtools", feature = "layers", feature = "workbench", feature = "icon", feature = "datatypes", feature = "elf", feature = "amigainput", feature = "asl", feature = "vulkan", feature = "reaction"))]
pub(crate) mod iface;
pub mod interrupt;
#[cfg(feature = "intuition")]
pub mod intuition;
#[cfg(feature = "reaction")]
pub mod reaction;
#[cfg(feature = "intuition")]
pub mod menu;
#[cfg(feature = "popupmenu")]
pub mod popup;
#[cfg(feature = "layers")]
pub mod layers;
#[cfg(feature = "datatypes")]
pub mod datatypes;
#[cfg(feature = "console")]
pub mod console;
#[cfg(feature = "elf")]
pub mod elf;
#[cfg(feature = "amigainput")]
pub mod input;
#[cfg(feature = "workbench")]
pub mod workbench;
pub mod io;
pub mod library;
pub mod list;
#[cfg(feature = "locale")]
pub mod locale;
pub mod mem;
#[cfg(feature = "bsdsocket")]
pub mod net;
pub mod msg;
pub mod named;
pub mod panic;
#[cfg(feature = "pci")]
pub mod pci;
pub mod pool;
pub mod port;
#[cfg(feature = "prefs_objects")]
pub mod prefs;
pub mod signal;
pub mod stack;
pub mod sync;
pub mod sysinfo;
pub mod tag;
pub mod task;
pub mod track;
#[cfg(feature = "timer")]
pub mod timer;
#[cfg(feature = "utility")]
pub mod utility;
#[cfg(feature = "virtio")]
pub mod virtio;
#[cfg(feature = "vulkan")]
pub mod vulkan;


pub use error::{AmigaError, Result};

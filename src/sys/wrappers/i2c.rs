//! II2C global(s) and convenience wrappers.
//!
//! Hand-written binding for i2c.library — bus-level access to I²C
//! peripherals (RTC, temperature sensors, etc.). The IFace exposes
//! Probe, Read/Write (plain and taglist variants), bus locking, and
//! controller-name/device-id introspection.

use crate::sys::types::*;
use crate::sys::interfaces::i2c::*;

// ---- II2C (I2CIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static II2C: *mut I2CIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut II2C: *mut I2CIFace = core::ptr::null_mut();

// ── Bus configuration / probing ──────────────────────────────

#[inline]
pub unsafe fn i2c_probe(address: u32) -> u32 {
    ((*II2C).Probe)(II2C, address)
}

#[inline]
pub unsafe fn i2c_set_speed(speed: u32) -> u32 {
    ((*II2C).SetSpeed)(II2C, speed)
}

// ── Read ─────────────────────────────────────────────────────

#[inline]
pub unsafe fn i2c_read(address: u32, reg: u32, buffer: *mut u8, length: u32) -> i32 {
    ((*II2C).Read)(II2C, address, reg, buffer, length)
}

#[inline]
pub unsafe fn i2c_read_tag_list(
    address: u32, reg: u32, buffer: *mut u8, length: u32, tags: *mut TagItem,
) -> i32 {
    ((*II2C).ReadTagList)(II2C, address, reg, buffer, length, tags)
}

// ── Write ────────────────────────────────────────────────────

#[inline]
pub unsafe fn i2c_write(address: u32, reg: u32, buffer: *mut u8, length: u32) -> i32 {
    ((*II2C).Write)(II2C, address, reg, buffer, length)
}

#[inline]
pub unsafe fn i2c_write_tag_list(
    address: u32, reg: u32, buffer: *mut u8, length: u32, tags: *mut TagItem,
) -> i32 {
    ((*II2C).WriteTagList)(II2C, address, reg, buffer, length, tags)
}

// ── Bus introspection / locking ──────────────────────────────

#[inline]
pub unsafe fn i2c_get_name() -> STRPTR {
    ((*II2C).GetName)(II2C)
}

#[inline]
pub unsafe fn i2c_get_device_id(address: u32) -> u32 {
    ((*II2C).GetDeviceId)(II2C, address)
}

#[inline]
pub unsafe fn i2c_lock() {
    ((*II2C).Lock)(II2C)
}

#[inline]
pub unsafe fn i2c_unlock() {
    ((*II2C).Unlock)(II2C)
}

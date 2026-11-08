//! IBattClock global(s) and convenience wrappers.
//!
//! Hand-written binding for battclock.resource — battery-backed
//! real-time clock chip (Amiga's RTC). On AmigaOne hardware this
//! talks to the i2c-attached RTC chip.

use crate::sys::types::*;
use crate::sys::interfaces::battclock::*;

// ---- IBattClock (BattClockIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IBattClock: *mut BattClockIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IBattClock: *mut BattClockIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn battclock_reset_batt_clock() {
    ((*IBattClock).ResetBattClock)(IBattClock)
}

#[inline]
pub unsafe fn battclock_read_batt_clock() -> u32 {
    ((*IBattClock).ReadBattClock)(IBattClock)
}

#[inline]
pub unsafe fn battclock_write_batt_clock(time: u32) {
    ((*IBattClock).WriteBattClock)(IBattClock, time)
}

// ── Battery-backed scratch RAM ───────────────────────────────

#[inline]
pub unsafe fn battclock_read_batt_clock_mem(offset: u32, length: u32) -> u32 {
    ((*IBattClock).ReadBattClockMem)(IBattClock, offset, length)
}

#[inline]
pub unsafe fn battclock_write_batt_clock_mem(offset: u32, length: u32, value: u32) {
    ((*IBattClock).WriteBattClockMem)(IBattClock, offset, length, value)
}

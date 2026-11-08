//! IDateBrowser global(s) and convenience wrappers.
//!
//! Hand-written ReAction date-gadget binding. Beyond the class
//! accessor, exposes Julian-calendar helpers used by the gadget
//! internally and useful for date arithmetic in user code.

use crate::sys::types::*;
use crate::sys::interfaces::datebrowser::*;

// ---- IDateBrowser (DateBrowserIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IDateBrowser: *mut DateBrowserIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IDateBrowser: *mut DateBrowserIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn datebrowser_datebrowser_get_class() -> *mut APTR {
    ((*IDateBrowser).DATEBROWSER_GetClass)(IDateBrowser)
}

// ── Julian-calendar helpers ──────────────────────────────────

#[inline]
pub unsafe fn datebrowser_julian_week_day(day: u16, month: u16, year: i32) -> u16 {
    ((*IDateBrowser).JulianWeekDay)(IDateBrowser, day, month, year)
}

#[inline]
pub unsafe fn datebrowser_julian_month_days(month: u16, year: i32) -> u16 {
    ((*IDateBrowser).JulianMonthDays)(IDateBrowser, month, year)
}

#[inline]
pub unsafe fn datebrowser_julian_leap_year(year: i32) -> u32 {
    ((*IDateBrowser).JulianLeapYear)(IDateBrowser, year)
}

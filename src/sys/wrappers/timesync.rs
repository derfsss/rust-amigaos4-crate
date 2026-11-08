//! ITimesync global(s) and convenience wrappers.
//!
//! Hand-written binding for timesync.library — NTP-style remote time
//! synchronisation.

use crate::sys::types::*;
use crate::sys::interfaces::timesync::*;

// ---- ITimesync (TimesyncIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static ITimesync: *mut TimesyncIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut ITimesync: *mut TimesyncIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn timesync_remote_sync_a(tag_list: *const TagItem) -> u32 {
    ((*ITimesync).RemoteSyncA)(ITimesync, tag_list)
}

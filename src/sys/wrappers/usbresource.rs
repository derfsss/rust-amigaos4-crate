//! IUSBResource global(s) and convenience wrappers.
//!
//! Hand-written binding for usbresource — the USB stack's central
//! registration point for host-controller drivers (HCDs) and
//! function-drivers (FDs). Many `Reserved<N>` slots are placeholders
//! and skipped by the audit.

use crate::sys::types::*;
use crate::sys::interfaces::usbresource::*;

// ---- IUSBResource (USBResourceIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IUSBResource: *mut USBResourceIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IUSBResource: *mut USBResourceIFace = core::ptr::null_mut();

// ── Function-driver registration ─────────────────────────────

#[inline]
pub unsafe fn usbresource_usbres_register_fda(tag_list: *mut TagItem) -> APTR {
    ((*IUSBResource).USBResRegisterFDA)(IUSBResource, tag_list)
}

#[inline]
pub unsafe fn usbresource_usbres_unregister_fd(handle: APTR) {
    ((*IUSBResource).USBResUnregisterFD)(IUSBResource, handle)
}

// ── Host-controller-driver registration ──────────────────────

#[inline]
pub unsafe fn usbresource_usbres_register_hcda(tag_list: *mut TagItem) -> APTR {
    ((*IUSBResource).USBResRegisterHCDA)(IUSBResource, tag_list)
}

#[inline]
pub unsafe fn usbresource_usbres_unregister_hcd(handle: APTR) {
    ((*IUSBResource).USBResUnregisterHCD)(IUSBResource, handle)
}

// ── Event notification ───────────────────────────────────────

#[inline]
pub unsafe fn usbresource_usbres_add_notify(signal_mask: u32, port: *mut MsgPort) -> APTR {
    ((*IUSBResource).USBResAddNotify)(IUSBResource, signal_mask, port)
}

#[inline]
pub unsafe fn usbresource_usbres_rem_notify(handle: APTR) {
    ((*IUSBResource).USBResRemNotify)(IUSBResource, handle)
}

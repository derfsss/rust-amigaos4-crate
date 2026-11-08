//! IPCI global(s) and convenience wrappers.
//!
//! Hand-written binding for expansion.library's PCI sub-interface —
//! device enumeration and PCI resource (memory/IO range) allocation.

use crate::sys::types::*;
use crate::sys::interfaces::pci::*;

// ---- IPCI (PCIIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IPCI: *mut PCIIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IPCI: *mut PCIIFace = core::ptr::null_mut();

// ── Device enumeration ───────────────────────────────────────

#[inline]
pub unsafe fn pci_find_device(tag_list: *const TagItem) -> *mut PCIDevice {
    ((*IPCI).FindDevice)(IPCI, tag_list)
}

#[inline]
pub unsafe fn pci_free_device(device: *mut PCIDevice) {
    ((*IPCI).FreeDevice)(IPCI, device)
}

// ── Resource allocation ──────────────────────────────────────

#[inline]
pub unsafe fn pci_alloc_resource(resource_type: u32, size: u32) -> *mut PCIResourceRange {
    ((*IPCI).AllocResource)(IPCI, resource_type, size)
}

#[inline]
pub unsafe fn pci_free_resource(resource: *mut PCIResourceRange) {
    ((*IPCI).FreeResource)(IPCI, resource)
}

//! IOpenFirmware global(s) and convenience wrappers.
//!
//! Hand-written binding for openfirmware.resource — IEEE 1275
//! Open Firmware device-tree access on PowerPC AmigaOne hardware.
//! Walks the firmware tree, queries properties, finds nodes by
//! name/path/device-type, plus the RTAS call gateway.

use crate::sys::types::*;
use crate::sys::interfaces::openfirmware::*;

// ---- IOpenFirmware (OpenFirmwareIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IOpenFirmware: *mut OpenFirmwareIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IOpenFirmware: *mut OpenFirmwareIFace = core::ptr::null_mut();

// ── RTAS gateway ─────────────────────────────────────────────

#[inline]
pub unsafe fn openfirmware_call_rtas(args: *mut u32) -> u32 {
    ((*IOpenFirmware).CallRTAS)(IOpenFirmware, args)
}

#[inline]
pub unsafe fn openfirmware_get_rtasservice(name: CONST_STRPTR) -> u32 {
    ((*IOpenFirmware).GetRTASService)(IOpenFirmware, name)
}

// ── Tree navigation ─────────────────────────────────────────

#[inline]
pub unsafe fn openfirmware_root() -> APTR {
    ((*IOpenFirmware).Root)(IOpenFirmware)
}

#[inline]
pub unsafe fn openfirmware_peer(node: APTR) -> APTR {
    ((*IOpenFirmware).Peer)(IOpenFirmware, node)
}

#[inline]
pub unsafe fn openfirmware_child(node: APTR) -> APTR {
    ((*IOpenFirmware).Child)(IOpenFirmware, node)
}

#[inline]
pub unsafe fn openfirmware_parent(node: APTR) -> APTR {
    ((*IOpenFirmware).Parent)(IOpenFirmware, node)
}

// ── Property access ─────────────────────────────────────────

#[inline]
pub unsafe fn openfirmware_find_prop(node: APTR, name: CONST_STRPTR) -> APTR {
    ((*IOpenFirmware).FindProp)(IOpenFirmware, node, name)
}

#[inline]
pub unsafe fn openfirmware_first_prop(node: APTR) -> APTR {
    ((*IOpenFirmware).FirstProp)(IOpenFirmware, node)
}

#[inline]
pub unsafe fn openfirmware_next_prop(node: APTR, prop: APTR) -> APTR {
    ((*IOpenFirmware).NextProp)(IOpenFirmware, node, prop)
}

#[inline]
pub unsafe fn openfirmware_get_prop_name(prop: APTR, buffer: STRPTR, max_len: u32) -> u32 {
    ((*IOpenFirmware).GetPropName)(IOpenFirmware, prop, buffer, max_len)
}

#[inline]
pub unsafe fn openfirmware_prop_data_size(prop: APTR) -> u32 {
    ((*IOpenFirmware).PropDataSize)(IOpenFirmware, prop)
}

#[inline]
pub unsafe fn openfirmware_get_prop_data(prop: APTR, buffer: APTR, max_size: u32) -> u32 {
    ((*IOpenFirmware).GetPropData)(IOpenFirmware, prop, buffer, max_size)
}

#[inline]
pub unsafe fn openfirmware_get_num_address_cells(node: APTR) -> u32 {
    ((*IOpenFirmware).GetNumAddressCells)(IOpenFirmware, node)
}

#[inline]
pub unsafe fn openfirmware_get_num_size_cells(node: APTR) -> u32 {
    ((*IOpenFirmware).GetNumSizeCells)(IOpenFirmware, node)
}

#[inline]
pub unsafe fn openfirmware_get_prop_data_long(
    node: APTR, name: CONST_STRPTR, index: u32, default_value: u32,
) -> u32 {
    ((*IOpenFirmware).GetPropDataLong)(IOpenFirmware, node, name, index, default_value)
}

// ── Find-node helpers ───────────────────────────────────────

#[inline]
pub unsafe fn openfirmware_find_node_by_str_prop(
    root: APTR, prop_name: CONST_STRPTR, value: CONST_STRPTR, match_partial: u32,
) -> APTR {
    ((*IOpenFirmware).FindNodeByStrProp)(IOpenFirmware, root, prop_name, value, match_partial)
}

#[inline]
pub unsafe fn openfirmware_find_node_by_name(root: APTR, name: CONST_STRPTR) -> APTR {
    ((*IOpenFirmware).FindNodeByName)(IOpenFirmware, root, name)
}

#[inline]
pub unsafe fn openfirmware_find_node_by_device_type(root: APTR, dev_type: CONST_STRPTR) -> APTR {
    ((*IOpenFirmware).FindNodeByDeviceType)(IOpenFirmware, root, dev_type)
}

#[inline]
pub unsafe fn openfirmware_find_compatible_node(root: APTR, compatible: CONST_STRPTR) -> APTR {
    ((*IOpenFirmware).FindCompatibleNode)(IOpenFirmware, root, compatible)
}

#[inline]
pub unsafe fn openfirmware_find_node_by_path(path: CONST_STRPTR) -> APTR {
    ((*IOpenFirmware).FindNodeByPath)(IOpenFirmware, path)
}

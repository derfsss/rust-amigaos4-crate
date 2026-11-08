//! IExpansion global(s) and convenience wrappers.
//!
//! Hand-written binding for expansion.library — Zorro/PCI device
//! discovery, boot-node registration, machine-info introspection.

use crate::sys::types::*;
use crate::sys::interfaces::expansion::*;

// ---- IExpansion (ExpansionIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IExpansion: *mut ExpansionIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IExpansion: *mut ExpansionIFace = core::ptr::null_mut();

// ── ConfigDev list management ────────────────────────────────

#[inline]
pub unsafe fn expansion_add_config_dev(config_dev: *mut ConfigDev) {
    ((*IExpansion).AddConfigDev)(IExpansion, config_dev)
}

#[inline]
pub unsafe fn expansion_alloc_config_dev() -> *mut ConfigDev {
    ((*IExpansion).AllocConfigDev)(IExpansion)
}

#[inline]
pub unsafe fn expansion_find_config_dev(start: *const ConfigDev, manufacturer: i32, product: i32) -> *mut ConfigDev {
    ((*IExpansion).FindConfigDev)(IExpansion, start, manufacturer, product)
}

#[inline]
pub unsafe fn expansion_free_config_dev(config_dev: *mut ConfigDev) {
    ((*IExpansion).FreeConfigDev)(IExpansion, config_dev)
}

#[inline]
pub unsafe fn expansion_rem_config_dev(config_dev: *mut ConfigDev) {
    ((*IExpansion).RemConfigDev)(IExpansion, config_dev)
}

// ── Board configuration ──────────────────────────────────────

#[inline]
pub unsafe fn expansion_config_board(board: APTR, config_dev: *mut ConfigDev) -> u32 {
    ((*IExpansion).ConfigBoard)(IExpansion, board, config_dev)
}

#[inline]
pub unsafe fn expansion_config_chain(base: APTR) -> u32 {
    ((*IExpansion).ConfigChain)(IExpansion, base)
}

// ── Board memory ─────────────────────────────────────────────

#[inline]
pub unsafe fn expansion_alloc_board_mem(slot_spec: u32) -> u32 {
    ((*IExpansion).AllocBoardMem)(IExpansion, slot_spec)
}

#[inline]
pub unsafe fn expansion_free_board_mem(start_slot: u32, slot_count: u32) {
    ((*IExpansion).FreeBoardMem)(IExpansion, start_slot, slot_count)
}

#[inline]
pub unsafe fn expansion_alloc_expansion_mem(num_slots: u32, slot_align: u32) -> APTR {
    ((*IExpansion).AllocExpansionMem)(IExpansion, num_slots, slot_align)
}

#[inline]
pub unsafe fn expansion_free_expansion_mem(start_slot: u32, num_slots: u32) {
    ((*IExpansion).FreeExpansionMem)(IExpansion, start_slot, num_slots)
}

// ── ExpansionROM byte/word I/O ───────────────────────────────

#[inline]
pub unsafe fn expansion_read_expansion_byte(board: CONST_APTR, offset: u32) -> u8 {
    ((*IExpansion).ReadExpansionByte)(IExpansion, board, offset)
}

#[inline]
pub unsafe fn expansion_read_expansion_rom(board: CONST_APTR, config_dev: *mut ConfigDev) -> u32 {
    ((*IExpansion).ReadExpansionRom)(IExpansion, board, config_dev)
}

#[inline]
pub unsafe fn expansion_write_expansion_byte(board: APTR, offset: u32, value: u8) {
    ((*IExpansion).WriteExpansionByte)(IExpansion, board, offset, value)
}

#[inline]
pub unsafe fn expansion_write_expansion_word(board: APTR, offset: u32, value: u16) {
    ((*IExpansion).WriteExpansionWord)(IExpansion, board, offset, value)
}

// ── Boot-node registration / DOS-node construction ───────────

#[inline]
pub unsafe fn expansion_add_boot_node(
    boot_priority: i32, flags: u32, device_node: APTR, config_dev: *mut ConfigDev,
) -> u32 {
    ((*IExpansion).AddBootNode)(IExpansion, boot_priority, flags, device_node, config_dev)
}

#[inline]
pub unsafe fn expansion_make_dos_node(parameter_packet: CONST_APTR) -> *mut DeviceNode {
    ((*IExpansion).MakeDosNode)(IExpansion, parameter_packet)
}

#[inline]
pub unsafe fn expansion_add_dos_node(
    boot_priority: i32, flags: u32, device_node: *mut DeviceNode,
) -> u32 {
    ((*IExpansion).AddDosNode)(IExpansion, boot_priority, flags, device_node)
}

// ── Driver-binding context ───────────────────────────────────

#[inline]
pub unsafe fn expansion_obtain_config_binding() {
    ((*IExpansion).ObtainConfigBinding)(IExpansion)
}

#[inline]
pub unsafe fn expansion_release_config_binding() {
    ((*IExpansion).ReleaseConfigBinding)(IExpansion)
}

#[inline]
pub unsafe fn expansion_set_current_binding(binding: *mut CurrentBinding, length: u32) {
    ((*IExpansion).SetCurrentBinding)(IExpansion, binding, length)
}

#[inline]
pub unsafe fn expansion_get_current_binding(binding: *const CurrentBinding, length: u32) -> u32 {
    ((*IExpansion).GetCurrentBinding)(IExpansion, binding, length)
}

// ── Machine info ─────────────────────────────────────────────

#[inline]
pub unsafe fn expansion_get_machine_info(tag_list: *const TagItem) -> u32 {
    ((*IExpansion).GetMachineInfo)(IExpansion, tag_list)
}

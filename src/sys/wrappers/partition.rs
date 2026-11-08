//! IPartition global(s) and convenience wrappers.
//!
//! Hand-written ReAction partition-class binding with node-list
//! helpers. Same shape as chooser / clicktab / radiobutton.

use crate::sys::types::*;
use crate::sys::interfaces::partition::*;

// ---- IPartition (PartitionIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IPartition: *mut PartitionIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IPartition: *mut PartitionIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn partition_partition_get_class() -> *mut APTR {
    ((*IPartition).PARTITION_GetClass)(IPartition)
}

// ── Partition-node lifecycle ─────────────────────────────────

#[inline]
pub unsafe fn partition_alloc_partition_node_a(tag_list: *mut TagItem) -> *mut Node {
    ((*IPartition).AllocPartitionNodeA)(IPartition, tag_list)
}

#[inline]
pub unsafe fn partition_free_partition_node(node: *mut Node) {
    ((*IPartition).FreePartitionNode)(IPartition, node)
}

#[inline]
pub unsafe fn partition_set_partition_node_attrs_a(node: *mut Node, tag_list: *mut TagItem) {
    ((*IPartition).SetPartitionNodeAttrsA)(IPartition, node, tag_list)
}

#[inline]
pub unsafe fn partition_get_partition_node_attrs_a(node: *mut Node, tag_list: *mut TagItem) {
    ((*IPartition).GetPartitionNodeAttrsA)(IPartition, node, tag_list)
}

//! SANA-II extensions (unofficial), version 1: the layouts and constants of
//! `include/devices/sana2ext.h`, for drivers that offer them and stacks
//! that use them. The rules (ownership, completion, abort, versioning) are
//! in `docs/devices/sana2ext.md`.
//!
//! - [`S2XQuery`] (`S2X_QUERY`): what a driver offers, with its list of
//!   capability items ([`S2XCapBatch`], [`S2XCapChecksum`],
//!   [`S2XCapSegment`], [`S2XCapMTU`], [`S2XCapModeration`],
//!   [`S2XCapFlowControl`], [`S2XCapRSS`]).
//! - [`S2XBatch`] of [`S2XFrame`]s (`S2X_READBATCH`, `S2X_WRITEBATCH`):
//!   several frames per request, on one queue, each frame a standard
//!   [`IOSana2Req`](crate::sana2::IOSana2Req) plus its items
//!   ([`S2XFrameChecksum`], [`S2XFrameSegment`], [`S2XFrameRSS`]).
//! - [`S2XConfigure`] (`S2X_CONFIGURE`): read or change the unit's settings
//!   ([`S2XSetMTU`], [`S2XSetModeration`], [`S2XSetFlowControl`],
//!   [`S2XSetRSS`]).
//!
//! Every item starts with an [`S2XItem`] header (a `MinNode`, a type and
//! the item's size, as `struct PCICapability`); a reader uses the types it
//! knows and steps over the others. [`MinList`] and [`items`] walk them.
//!
//! Portable by construction: every field lies at a multiple of its own
//! size and every structure is a multiple of the pointer size, so the
//! layout is the same with the SDK's pack(2) and with natural alignment,
//! with 32- and 64-bit pointers, on any byte order (fields are host order,
//! frame data as on the wire). The compile-time checks below are written in
//! the pointer size and run on every target (the host build checks 64-bit);
//! `tests/layout/sana2ext.c` checks the C side, and
//! `tests/sana2ext_consts.rs` the constants against the header.
//!
//! Exec's message passing is the only hand-over between a driver and a
//! stack (also between CPUs): a driver writes a batch before ReplyMsg and
//! never after; a stack reads it only after it has the reply.

use core::ptr::{addr_of, addr_of_mut, null_mut};

use crate::sana2::{IOSana2Req, MinNode};

/// `S2XQuery::s2xq_Magic` from a driver that implements these extensions:
/// 'S2XT'.
pub const S2X_MAGIC: u32 = 0x5332_5854;
/// The version this module describes.
pub const S2X_VERSION: u16 = 1;

/// Commands; `ios2_StatData` points to the command's structure.
pub const S2X_QUERY: u16 = 0xC100;
pub const S2X_READBATCH: u16 = 0xC101;
pub const S2X_WRITEBATCH: u16 = 0xC102;
pub const S2X_CONFIGURE: u16 = 0xC103;

/// Capability item types (S2X_QUERY's list).
pub const S2XCAP_BATCH: u32 = 0x0001;
pub const S2XCAP_RXCHECKSUM: u32 = 0x0002;
pub const S2XCAP_TXCHECKSUM: u32 = 0x0003;
pub const S2XCAP_SEGMENT: u32 = 0x0004;
pub const S2XCAP_MTU: u32 = 0x0005;
pub const S2XCAP_MODERATION: u32 = 0x0006;
pub const S2XCAP_FLOWCONTROL: u32 = 0x0007;
pub const S2XCAP_RSS: u32 = 0x0008;
/// Frame item types (S2XFrame's list).
pub const S2XFRAME_CHECKSUM: u32 = 0x1001;
pub const S2XFRAME_SEGMENT: u32 = 0x1002;
pub const S2XFRAME_RSS: u32 = 0x1003;
/// Setting item types (S2X_CONFIGURE's list).
pub const S2XSET_MTU: u32 = 0x2001;
pub const S2XSET_MODERATION: u32 = 0x2002;
pub const S2XSET_FLOWCONTROL: u32 = 0x2003;
pub const S2XSET_RSS: u32 = 0x2004;
/// Set in a type that belongs to one driver or stack: others ignore it.
pub const S2XITEM_PRIVATE: u32 = 0x8000_0000;

/// `S2XQuery::s2xq_Features`: one bit per capability item present.
pub const S2XF_BATCH: u32 = 1 << S2XCAP_BATCH;
pub const S2XF_RXCHECKSUM: u32 = 1 << S2XCAP_RXCHECKSUM;
pub const S2XF_TXCHECKSUM: u32 = 1 << S2XCAP_TXCHECKSUM;
pub const S2XF_SEGMENT: u32 = 1 << S2XCAP_SEGMENT;
pub const S2XF_MTU: u32 = 1 << S2XCAP_MTU;
pub const S2XF_MODERATION: u32 = 1 << S2XCAP_MODERATION;
pub const S2XF_FLOWCONTROL: u32 = 1 << S2XCAP_FLOWCONTROL;
pub const S2XF_RSS: u32 = 1 << S2XCAP_RSS;

/// Protocols (S2XCapChecksum, S2XCapSegment, S2XFrameSegment).
pub const S2XCSUM_IPV4: u32 = 0x0001;
pub const S2XCSUM_TCPV4: u32 = 0x0002;
pub const S2XCSUM_UDPV4: u32 = 0x0004;
pub const S2XCSUM_TCPV6: u32 = 0x0008;
pub const S2XCSUM_UDPV6: u32 = 0x0010;

/// S2XFrameChecksum flags: receive results (set by the driver) ...
pub const S2XCS_IP_GOOD: u32 = 0x0001;
pub const S2XCS_IP_BAD: u32 = 0x0002;
pub const S2XCS_L4_GOOD: u32 = 0x0004;
pub const S2XCS_L4_BAD: u32 = 0x0008;
/// ... and send requests (set by the stack).
pub const S2XCS_FILL_IP: u32 = 0x0100;
pub const S2XCS_FILL_L4: u32 = 0x0200;

/// S2XCapModeration flags: which directions can be set.
pub const S2XMOD_RX: u32 = 0x0001;
pub const S2XMOD_TX: u32 = 0x0002;

/// Flow-control modes (S2XCapFlowControl, S2XSetFlowControl).
pub const S2XFC_RX: u32 = 0x0001;
pub const S2XFC_TX: u32 = 0x0002;
pub const S2XFC_AUTO: u32 = 0x0004;

/// RSS hash types (S2XCapRSS, S2XSetRSS, S2XFrameRSS).
pub const S2XRSS_IPV4: u32 = 0x0001;
pub const S2XRSS_TCPV4: u32 = 0x0002;
pub const S2XRSS_UDPV4: u32 = 0x0004;
pub const S2XRSS_IPV6: u32 = 0x0008;
pub const S2XRSS_TCPV6: u32 = 0x0010;
pub const S2XRSS_UDPV6: u32 = 0x0020;

/// S2XConfigure flags.
pub const S2XCF_READ: u32 = 0x0001;

/// `struct MinList` (exec/lists.h).
#[repr(C, packed(2))]
pub struct MinList {
    pub mlh_Head: *mut MinNode,
    pub mlh_Tail: *mut MinNode,
    pub mlh_TailPred: *mut MinNode,
}

impl MinList {
    /// Make the list empty (exec's NewMinList).
    ///
    /// # Safety
    /// `l` is valid for writes and does not move while the list is used.
    pub unsafe fn init(l: *mut MinList) {
        // SAFETY: the caller's contract; packed fields through raw pointers.
        unsafe {
            addr_of_mut!((*l).mlh_Head).write_unaligned(addr_of_mut!((*l).mlh_Tail) as *mut MinNode);
            addr_of_mut!((*l).mlh_Tail).write_unaligned(null_mut());
            addr_of_mut!((*l).mlh_TailPred).write_unaligned(addr_of_mut!((*l).mlh_Head) as *mut MinNode);
        }
    }

    /// Append `n` (exec's AddTail).
    ///
    /// # Safety
    /// `l` was set up with [`MinList::init`]; `n` is valid, in no list, and
    /// stays put while linked.
    pub unsafe fn add_tail(l: *mut MinList, n: *mut MinNode) {
        // SAFETY: the caller's contract: a well-formed list and a free node.
        unsafe {
            let tail = addr_of_mut!((*l).mlh_Tail) as *mut MinNode;
            let pred = addr_of!((*l).mlh_TailPred).read_unaligned();
            addr_of_mut!((*n).mln_Succ).write_unaligned(tail);
            addr_of_mut!((*n).mln_Pred).write_unaligned(pred);
            addr_of_mut!((*pred).mln_Succ).write_unaligned(n);
            addr_of_mut!((*l).mlh_TailPred).write_unaligned(n);
        }
    }

    /// Whether the list has no node.
    ///
    /// # Safety
    /// `l` is a well-formed list.
    pub unsafe fn is_empty(l: *const MinList) -> bool {
        // SAFETY: the caller's contract.
        unsafe { addr_of!((*l).mlh_TailPred).read_unaligned() == addr_of!((*l).mlh_Head) as *mut MinNode }
    }
}

/// The nodes of a list, first to last. The nodes must stay linked and
/// valid while the iterator runs.
pub struct Nodes {
    next: *mut MinNode,
}

impl Iterator for Nodes {
    type Item = *mut MinNode;
    fn next(&mut self) -> Option<*mut MinNode> {
        let n = self.next;
        if n.is_null() {
            return None;
        }
        // SAFETY: `nodes`'s contract: every node of the list is valid; the
        // tail sentinel's successor is NULL.
        let succ = unsafe { addr_of!((*n).mln_Succ).read_unaligned() };
        if succ.is_null() {
            self.next = null_mut();
            return None;
        }
        self.next = succ;
        Some(n)
    }
}

/// The nodes of `l`.
///
/// # Safety
/// `l` is a well-formed list whose nodes stay valid while the iterator is
/// used.
pub unsafe fn nodes(l: *const MinList) -> Nodes {
    // SAFETY: the caller's contract.
    Nodes { next: unsafe { addr_of!((*l).mlh_Head).read_unaligned() } }
}

/// The items of `l` (each node is an [`S2XItem`]'s first member).
///
/// # Safety
/// As [`nodes`], and every node is the start of an item.
pub unsafe fn items(l: *const MinList) -> impl Iterator<Item = *mut S2XItem> {
    // SAFETY: the caller's contract.
    unsafe { nodes(l) }.map(|n| n as *mut S2XItem)
}

/// The first item of type `ty` in `l` that is at least `size_of::<T>()`
/// bytes. A smaller one (from an older version, with fields missing) is
/// skipped; a caller that can use a shorter item walks [`items`] itself.
///
/// # Safety
/// As [`items`]; `T` is the item structure `ty` names.
pub unsafe fn find<T>(l: *const MinList, ty: u32) -> Option<*mut T> {
    // SAFETY: the caller's contract.
    for it in unsafe { items(l) } {
        // SAFETY: a valid item (the caller's contract).
        let (t, s) = unsafe { (addr_of!((*it).s2xi_Type).read_unaligned(), addr_of!((*it).s2xi_Size).read_unaligned()) };
        if t == ty && s as usize >= core::mem::size_of::<T>() {
            return Some(it as *mut T);
        }
    }
    None
}

/// Every item's header.
#[repr(C, packed(2))]
pub struct S2XItem {
    pub s2xi_Node: MinNode,
    pub s2xi_Type: u32,
    /// Bytes of the whole item, header included.
    pub s2xi_Size: u32,
}

impl S2XItem {
    /// A header for an item of type `ty` that is `size` bytes (unlinked).
    pub const fn new(ty: u32, size: usize) -> Self {
        S2XItem { s2xi_Node: MinNode { mln_Succ: null_mut(), mln_Pred: null_mut() }, s2xi_Type: ty, s2xi_Size: size as u32 }
    }
}

/// S2X_QUERY's answer.
#[repr(C, packed(2))]
pub struct S2XQuery {
    /// Bytes the caller provides.
    pub s2xq_Size: u32,
    pub s2xq_Magic: u32,
    pub s2xq_Version: u16,
    pub s2xq_Revision: u16,
    pub s2xq_Features: u32,
    /// The driver's capability items, read only, valid until CloseDevice.
    pub s2xq_Capabilities: *const MinList,
}

/// S2XCAP_BATCH.
#[repr(C, packed(2))]
pub struct S2XCapBatch {
    pub s2xcb_Header: S2XItem,
    pub s2xcb_MaxFrames: u32,
    pub s2xcb_Flags: u32,
}

/// S2XCAP_RXCHECKSUM / S2XCAP_TXCHECKSUM.
#[repr(C, packed(2))]
pub struct S2XCapChecksum {
    pub s2xcc_Header: S2XItem,
    pub s2xcc_Protocols: u32,
    /// 0.
    pub s2xcc_Reserved: u32,
}

/// S2XCAP_SEGMENT.
#[repr(C, packed(2))]
pub struct S2XCapSegment {
    pub s2xcs_Header: S2XItem,
    pub s2xcs_Protocols: u32,
    pub s2xcs_MaxSize: u32,
    pub s2xcs_MaxSegments: u16,
    pub s2xcs_MinSegSize: u16,
    /// 0.
    pub s2xcs_Reserved: u32,
}

/// S2XCAP_MTU: the MTUs S2XSET_MTU accepts (as Sana2DeviceQuery's MTU).
#[repr(C, packed(2))]
pub struct S2XCapMTU {
    pub s2xcm_Header: S2XItem,
    pub s2xcm_MinMTU: u32,
    pub s2xcm_MaxMTU: u32,
}

/// S2XCAP_MODERATION.
#[repr(C, packed(2))]
pub struct S2XCapModeration {
    pub s2xco_Header: S2XItem,
    /// Longest interval; 0 (off) is always allowed.
    pub s2xco_MaxMicros: u32,
    /// S2XMOD_... bits.
    pub s2xco_Flags: u32,
}

/// S2XCAP_FLOWCONTROL.
#[repr(C, packed(2))]
pub struct S2XCapFlowControl {
    pub s2xcf_Header: S2XItem,
    /// S2XFC_... modes the card supports.
    pub s2xcf_Modes: u32,
    /// 0.
    pub s2xcf_Reserved: u32,
}

/// S2XCAP_RSS: receive-side scaling (Toeplitz hash, indirection table).
#[repr(C, packed(2))]
pub struct S2XCapRSS {
    pub s2xcr_Header: S2XItem,
    /// S2XRSS_... the card can hash.
    pub s2xcr_HashTypes: u32,
    /// Receive queues.
    pub s2xcr_MaxQueues: u16,
    pub s2xcr_KeySize: u16,
    pub s2xcr_TableSize: u16,
    /// Transmit queues.
    pub s2xcr_MaxTxQueues: u16,
    /// 0.
    pub s2xcr_Reserved: u32,
}

/// One frame of a batch.
#[repr(C, packed(2))]
pub struct S2XFrame {
    pub s2xf_Node: MinNode,
    /// The frame's standard request (an opened copy of the batch's).
    pub s2xf_Req: *mut IOSana2Req,
    /// The frame's items (may be empty).
    pub s2xf_Items: MinList,
    /// Bytes of this structure.
    pub s2xf_Size: u32,
    /// 0.
    pub s2xf_Reserved: u32,
}

/// S2XFRAME_CHECKSUM.
#[repr(C, packed(2))]
pub struct S2XFrameChecksum {
    pub s2xfc_Header: S2XItem,
    pub s2xfc_Flags: u32,
    pub s2xfc_Start: u16,
    pub s2xfc_Offset: u16,
}

/// S2XFRAME_SEGMENT.
#[repr(C, packed(2))]
pub struct S2XFrameSegment {
    pub s2xfs_Header: S2XItem,
    pub s2xfs_Protocol: u32,
    pub s2xfs_SegSize: u16,
    pub s2xfs_HdrLen: u16,
}

/// S2XFRAME_RSS (receive): the card's hash.
#[repr(C, packed(2))]
pub struct S2XFrameRSS {
    pub s2xfr_Header: S2XItem,
    pub s2xfr_Hash: u32,
    /// The S2XRSS_... type used; 0: not hashed.
    pub s2xfr_HashType: u32,
}

/// S2X_READBATCH / S2X_WRITEBATCH.
#[repr(C, packed(2))]
pub struct S2XBatch {
    /// Bytes of this structure.
    pub s2xb_Size: u32,
    pub s2xb_Count: u32,
    pub s2xb_Frames: MinList,
    /// Set by the driver: frames filled (read) or taken (write).
    pub s2xb_Done: u32,
    /// Receive / transmit queue; 0 without RSS.
    pub s2xb_Queue: u32,
}

/// S2X_CONFIGURE.
#[repr(C, packed(2))]
pub struct S2XConfigure {
    /// Bytes of this structure.
    pub s2xc_Size: u32,
    /// S2XCF_... bits.
    pub s2xc_Flags: u32,
    /// S2XSet... items.
    pub s2xc_Items: MinList,
    /// Set by the driver on a refusal: the item it refused; else NULL.
    pub s2xc_Failed: *mut S2XItem,
}

/// S2XSET_MTU.
#[repr(C, packed(2))]
pub struct S2XSetMTU {
    pub s2xsm_Header: S2XItem,
    pub s2xsm_MTU: u32,
    /// 0.
    pub s2xsm_Reserved: u32,
}

/// S2XSET_MODERATION: microseconds between interrupts; 0 is off.
#[repr(C, packed(2))]
pub struct S2XSetModeration {
    pub s2xso_Header: S2XItem,
    pub s2xso_RxMicros: u32,
    pub s2xso_TxMicros: u32,
}

/// S2XSET_FLOWCONTROL.
#[repr(C, packed(2))]
pub struct S2XSetFlowControl {
    pub s2xsf_Header: S2XItem,
    /// S2XFC_... modes; 0 is off.
    pub s2xsf_Modes: u32,
    /// 0.
    pub s2xsf_Reserved: u32,
}

/// S2XSET_RSS: Queues 1 turns RSS off; Key and Table may be NULL.
#[repr(C, packed(2))]
pub struct S2XSetRSS {
    pub s2xsr_Header: S2XItem,
    pub s2xsr_HashTypes: u32,
    pub s2xsr_Queues: u16,
    pub s2xsr_TableSize: u16,
    /// KeySize bytes, or NULL.
    pub s2xsr_Key: *mut u8,
    /// TableSize queue numbers, or NULL.
    pub s2xsr_Table: *mut u8,
}

// The header's layout rule, checked for this target's pointer size (the
// host build checks 64-bit pointers, the AmigaOS build 32-bit): every field
// at a multiple of its own size, every structure a multiple of the pointer
// size, so pack(2) and natural alignment give the same layout.
const _: () = {
    use core::mem::{offset_of, size_of};
    const P: usize = size_of::<usize>();
    assert!(size_of::<MinNode>() == 2 * P && size_of::<MinList>() == 3 * P);
    assert!(size_of::<S2XItem>() == 2 * P + 8);
    assert!(offset_of!(S2XItem, s2xi_Type) == 2 * P && offset_of!(S2XItem, s2xi_Size) == 2 * P + 4);
    assert!(size_of::<S2XQuery>() == 16 + P);
    assert!(offset_of!(S2XQuery, s2xq_Version) == 8 && offset_of!(S2XQuery, s2xq_Features) == 12 && offset_of!(S2XQuery, s2xq_Capabilities) == 16);
    const H: usize = 2 * P + 8;
    assert!(size_of::<S2XCapBatch>() == H + 8 && offset_of!(S2XCapBatch, s2xcb_MaxFrames) == H);
    assert!(size_of::<S2XCapChecksum>() == H + 8 && offset_of!(S2XCapChecksum, s2xcc_Protocols) == H);
    assert!(size_of::<S2XCapSegment>() == H + 16 && offset_of!(S2XCapSegment, s2xcs_MaxSegments) == H + 8);
    assert!(size_of::<S2XFrame>() == 6 * P + 8);
    assert!(offset_of!(S2XFrame, s2xf_Req) == 2 * P && offset_of!(S2XFrame, s2xf_Items) == 3 * P && offset_of!(S2XFrame, s2xf_Size) == 6 * P);
    assert!(size_of::<S2XFrameChecksum>() == H + 8 && offset_of!(S2XFrameChecksum, s2xfc_Start) == H + 4 && offset_of!(S2XFrameChecksum, s2xfc_Offset) == H + 6);
    assert!(size_of::<S2XFrameSegment>() == H + 8 && offset_of!(S2XFrameSegment, s2xfs_SegSize) == H + 4);
    assert!(size_of::<S2XBatch>() == 16 + 3 * P);
    assert!(offset_of!(S2XBatch, s2xb_Count) == 4 && offset_of!(S2XBatch, s2xb_Frames) == 8 && offset_of!(S2XBatch, s2xb_Done) == 8 + 3 * P);
    assert!(offset_of!(S2XBatch, s2xb_Queue) == 12 + 3 * P);
    assert!(size_of::<S2XCapMTU>() == H + 8 && offset_of!(S2XCapMTU, s2xcm_MaxMTU) == H + 4);
    assert!(size_of::<S2XCapModeration>() == H + 8 && offset_of!(S2XCapModeration, s2xco_Flags) == H + 4);
    assert!(size_of::<S2XCapFlowControl>() == H + 8);
    assert!(size_of::<S2XCapRSS>() == H + 16 && offset_of!(S2XCapRSS, s2xcr_MaxQueues) == H + 4);
    assert!(offset_of!(S2XCapRSS, s2xcr_KeySize) == H + 6 && offset_of!(S2XCapRSS, s2xcr_TableSize) == H + 8 && offset_of!(S2XCapRSS, s2xcr_MaxTxQueues) == H + 10 && offset_of!(S2XCapRSS, s2xcr_Reserved) == H + 12);
    assert!(size_of::<S2XFrameRSS>() == H + 8 && offset_of!(S2XFrameRSS, s2xfr_HashType) == H + 4);
    assert!(size_of::<S2XConfigure>() == 8 + 4 * P);
    assert!(offset_of!(S2XConfigure, s2xc_Items) == 8 && offset_of!(S2XConfigure, s2xc_Failed) == 8 + 3 * P);
    assert!(size_of::<S2XSetMTU>() == H + 8 && size_of::<S2XSetModeration>() == H + 8 && size_of::<S2XSetFlowControl>() == H + 8);
    assert!(offset_of!(S2XSetModeration, s2xso_TxMicros) == H + 4);
    assert!(size_of::<S2XSetRSS>() == H + 8 + 2 * P && offset_of!(S2XSetRSS, s2xsr_Queues) == H + 4);
    assert!(offset_of!(S2XSetRSS, s2xsr_TableSize) == H + 6 && offset_of!(S2XSetRSS, s2xsr_Key) == H + 8 && offset_of!(S2XSetRSS, s2xsr_Table) == H + 8 + P);
    let sizes = [size_of::<S2XItem>(), size_of::<S2XQuery>(), size_of::<S2XCapBatch>(), size_of::<S2XCapChecksum>(), size_of::<S2XCapSegment>(), size_of::<S2XFrame>(), size_of::<S2XFrameChecksum>(), size_of::<S2XFrameSegment>(), size_of::<S2XBatch>(),
        size_of::<S2XCapMTU>(), size_of::<S2XCapModeration>(), size_of::<S2XCapFlowControl>(), size_of::<S2XCapRSS>(), size_of::<S2XFrameRSS>(),
        size_of::<S2XConfigure>(), size_of::<S2XSetMTU>(), size_of::<S2XSetModeration>(), size_of::<S2XSetFlowControl>(), size_of::<S2XSetRSS>()];
    let mut i = 0;
    while i < sizes.len() {
        assert!(sizes[i] % P == 0);
        i += 1;
    }
};

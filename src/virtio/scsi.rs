//! virtio-scsi (virtio 1.x section 5.6): PCI IDs, feature bits, the device
//! configuration layout, the request and response headers of the request
//! queue, LUN addressing, and the response codes.
//!
//! Queues: 0 control, 1 event, 2 the first request queue (`num_queues`
//! request queues follow from 2).
//!
//! Byte order: with the modern interface (`F_VERSION_1`) the multi-byte
//! fields the device reads or writes are little-endian; with the legacy
//! interface they are in the guest's order (big-endian here). Byte fields
//! and arrays (`lun`, `cdb`, `sense`) need no conversion; `id` is the
//! driver's own cookie and comes back as it was written.

/// PCI device ID, transitional (legacy and modern interfaces).
pub const PCI_ID_TRANSITIONAL: u32 = 0x1004;
/// PCI device ID, modern only.
pub const PCI_ID_MODERN: u32 = 0x1048;

pub const CONTROL_QUEUE: u16 = 0;
pub const EVENT_QUEUE: u16 = 1;
pub const REQUEST_QUEUE: u16 = 2;

// ---- Feature bits (section 5.6.3) ----

/// A request may carry data in both directions.
pub const F_INOUT: u64 = 1 << 0;
/// The device reports hot-plug and hot-unplug on the event queue.
pub const F_HOTPLUG: u64 = 1 << 1;
/// The device reports parameter changes (capacity) on the event queue.
pub const F_CHANGE: u64 = 1 << 2;
/// T10 protection information in the request header.
pub const F_T10_PI: u64 = 1 << 3;

// ---- Device configuration (section 5.6.4), byte offsets ----

pub const CFG_NUM_QUEUES: u32 = 0;
pub const CFG_SEG_MAX: u32 = 4;
pub const CFG_MAX_SECTORS: u32 = 8;
pub const CFG_CMD_PER_LUN: u32 = 12;
pub const CFG_EVENT_INFO_SIZE: u32 = 16;
pub const CFG_SENSE_SIZE: u32 = 20;
pub const CFG_CDB_SIZE: u32 = 24;
pub const CFG_MAX_CHANNEL: u32 = 28;
pub const CFG_MAX_TARGET: u32 = 30;
pub const CFG_MAX_LUN: u32 = 32;

/// The sizes this driver's headers use: the defaults the device starts with.
pub const CDB_SIZE: usize = 32;
pub const SENSE_SIZE: usize = 96;

// ---- Request queue headers (section 5.6.6.1) ----

/// `struct virtio_scsi_req_cmd` without T10 PI: device-readable, before the
/// data the device reads.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ReqCmd {
    pub lun: [u8; 8],
    /// The driver's cookie, returned unchanged.
    pub id: u64,
    pub task_attr: u8,
    pub prio: u8,
    pub crn: u8,
    pub cdb: [u8; CDB_SIZE],
}
const _: () = assert!(core::mem::size_of::<ReqCmd>() == 51);

/// `struct virtio_scsi_resp_cmd`: device-writable, before the data the
/// device writes.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct RespCmd {
    pub sense_len: u32,
    /// Bytes of the data buffer the device did not use.
    pub residual: u32,
    pub status_qualifier: u16,
    /// The SCSI status byte (0 GOOD, 2 CHECK CONDITION, ...).
    pub status: u8,
    /// The transport's answer (`S_OK` ...).
    pub response: u8,
    pub sense: [u8; SENSE_SIZE],
}
const _: () = assert!(core::mem::size_of::<RespCmd>() == 108);

impl ReqCmd {
    pub const fn zeroed() -> Self {
        ReqCmd { lun: [0; 8], id: 0, task_attr: TASK_SIMPLE, prio: 0, crn: 0, cdb: [0; CDB_SIZE] }
    }
}

impl RespCmd {
    pub const fn zeroed() -> Self {
        RespCmd { sense_len: 0, residual: 0, status_qualifier: 0, status: 0, response: 0, sense: [0; SENSE_SIZE] }
    }
}

/// The single-level LUN address of `target`/`lun` (section 5.6.6.1:
/// byte 0 is 1, byte 1 the target, bytes 2-3 the LUN with 0x40 set).
pub const fn lun_address(target: u8, lun: u16) -> [u8; 8] {
    [1, target, 0x40 | ((lun >> 8) as u8 & 0x3f), lun as u8, 0, 0, 0, 0]
}

// ---- Task attributes ----

pub const TASK_SIMPLE: u8 = 0;
pub const TASK_ORDERED: u8 = 1;
pub const TASK_HEAD: u8 = 2;
pub const TASK_ACA: u8 = 3;

// ---- Response codes (section 5.6.6.1.1) ----

pub const S_OK: u8 = 0;
pub const S_OVERRUN: u8 = 1;
pub const S_ABORTED: u8 = 2;
pub const S_BAD_TARGET: u8 = 3;
pub const S_RESET: u8 = 4;
pub const S_BUSY: u8 = 5;
pub const S_TRANSPORT_FAILURE: u8 = 6;
pub const S_TARGET_FAILURE: u8 = 7;
pub const S_NEXUS_FAILURE: u8 = 8;
pub const S_FAILURE: u8 = 9;

/// A response code's name, for logs.
pub const fn response_name(r: u8) -> &'static str {
    match r {
        S_OK => "OK",
        S_OVERRUN => "OVERRUN",
        S_ABORTED => "ABORTED",
        S_BAD_TARGET => "BAD_TARGET",
        S_RESET => "RESET",
        S_BUSY => "BUSY",
        S_TRANSPORT_FAILURE => "TRANSPORT_FAILURE",
        S_TARGET_FAILURE => "TARGET_FAILURE",
        S_NEXUS_FAILURE => "NEXUS_FAILURE",
        S_FAILURE => "FAILURE",
        _ => "?",
    }
}

// ---- Event queue (section 5.6.6.3) ----

pub const T_NO_EVENT: u32 = 0;
pub const T_TRANSPORT_RESET: u32 = 1;
pub const T_ASYNC_NOTIFY: u32 = 2;
pub const T_PARAM_CHANGE: u32 = 3;
pub const T_EVENTS_MISSED: u32 = 0x8000_0000;
pub const EVT_RESET_HARD: u32 = 0;
pub const EVT_RESET_RESCAN: u32 = 1;
pub const EVT_RESET_REMOVED: u32 = 2;

// ---- Splitting a request into commands ----

/// One command of a planned request: its place in the request and the DMA
/// blocks it moves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedCmd {
    /// Byte offset of this command's data in the request.
    pub offset: usize,
    /// Bytes (a whole number of blocks).
    pub len: usize,
    /// (physical address, length) of each data segment, in order.
    pub segs: alloc::vec::Vec<(u64, u32)>,
}

/// Split a request whose data is the DMA blocks `entries` (in order, a
/// whole number of `block` bytes in all) into commands of at most
/// `max_segs` segments and `max_bytes` bytes (a multiple of `block`), each a
/// whole number of blocks. A DMA block may be split across two commands.
///
/// None when the blocks are so fragmented that `max_segs` of them hold less
/// than one block (the caller bounces), or the arguments make no sense.
pub fn plan_commands(entries: &[(u64, u32)], block: usize, max_segs: usize, max_bytes: usize) -> Option<alloc::vec::Vec<PlannedCmd>> {
    use alloc::collections::VecDeque;
    use alloc::vec::Vec;
    if block == 0 || max_segs == 0 || max_bytes < block || max_bytes % block != 0 {
        return None;
    }
    let total: usize = entries.iter().map(|&(_, l)| l as usize).sum();
    if total % block != 0 {
        return None;
    }
    let mut queue: VecDeque<(u64, usize)> = entries.iter().filter(|&&(_, l)| l > 0).map(|&(a, l)| (a, l as usize)).collect();
    let mut cmds = Vec::new();
    let mut offset = 0usize;
    while !queue.is_empty() {
        let mut segs: Vec<(u64, usize)> = Vec::new();
        let mut len = 0usize;
        while segs.len() < max_segs && len < max_bytes {
            let Some((a, l)) = queue.pop_front() else { break };
            let take = l.min(max_bytes - len);
            segs.push((a, take));
            len += take;
            if take < l {
                queue.push_front((a + take as u64, l - take));
            }
        }
        // Back off to a whole number of blocks; the rest starts the next command.
        let mut excess = len % block;
        while excess > 0 {
            let (a, l) = segs.pop()?;
            if l <= excess {
                queue.push_front((a, l));
                excess -= l;
                len -= l;
            } else {
                segs.push((a, l - excess));
                queue.push_front((a + (l - excess) as u64, excess));
                len -= excess;
                excess = 0;
            }
        }
        if len == 0 {
            return None;
        }
        cmds.push(PlannedCmd { offset, len, segs: segs.into_iter().map(|(a, l)| (a, l as u32)).collect() });
        offset += len;
    }
    Some(cmds)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every planned command is a whole number of blocks, within the
    /// limits, and the segments in order are exactly the input bytes.
    fn check(entries: &[(u64, u32)], block: usize, max_segs: usize, max_bytes: usize) -> alloc::vec::Vec<PlannedCmd> {
        let cmds = plan_commands(entries, block, max_segs, max_bytes).expect("a plan");
        let mut flat_in = alloc::vec::Vec::new();
        for &(a, l) in entries {
            for i in 0..l as u64 {
                flat_in.push(a + i);
            }
        }
        let mut flat_out = alloc::vec::Vec::new();
        let mut offset = 0;
        for c in &cmds {
            assert_eq!(c.offset, offset);
            assert!(c.len % block == 0 && c.len > 0 && c.len <= max_bytes, "{:?}", c);
            assert!(c.segs.len() <= max_segs, "{:?}", c);
            assert_eq!(c.segs.iter().map(|&(_, l)| l as usize).sum::<usize>(), c.len);
            for &(a, l) in &c.segs {
                for i in 0..l as u64 {
                    flat_out.push(a + i);
                }
            }
            offset += c.len;
        }
        assert_eq!(flat_in, flat_out);
        cmds
    }

    #[test]
    fn contiguous_request_splits_at_max_bytes() {
        let c = check(&[(0x10_0000, 3 << 20)], 512, 254, 1 << 20);
        assert_eq!(c.len(), 3);
        assert_eq!(c[2].segs, alloc::vec![(0x10_0000 + (2 << 20), 1 << 20)]);
    }

    #[test]
    fn pages_split_at_max_segs() {
        let pages: alloc::vec::Vec<(u64, u32)> = (0..10).map(|i| (0x100_0000 + 0x3000 * i, 4096)).collect();
        let c = check(&pages, 512, 4, 1 << 20);
        assert_eq!(c.iter().map(|c| c.segs.len()).collect::<alloc::vec::Vec<_>>(), [4, 4, 2]);
    }

    #[test]
    fn ragged_blocks_carry_across_commands() {
        // 1000 + 24 = 1024 = two blocks, but at most two segments a command.
        let e = [(0x1000, 1000), (0x9000, 24), (0x2000, 1000), (0xa000, 24)];
        check(&e, 512, 2, 1 << 20);
        check(&e, 512, 3, 1024);
        check(&[(0x1000, 700), (0x5000, 700), (0x9000, 648)], 512, 2, 4096);
    }

    #[test]
    fn four_k_blocks() {
        check(&[(0x1000, 6144), (0x8000, 2048)], 4096, 2, 8192);
    }

    #[test]
    fn hopeless_fragmentation_is_refused() {
        // Two segments a command can never hold a 512-byte block.
        let e: alloc::vec::Vec<(u64, u32)> = (0..8).map(|i| (0x1000 * i, 128)).collect();
        assert_eq!(plan_commands(&e, 512, 2, 1 << 20), None);
        assert_eq!(plan_commands(&[(0, 1000)], 512, 4, 4096), None); // not whole blocks
    }

    #[test]
    fn lun_addresses() {
        assert_eq!(lun_address(0, 0), [1, 0, 0x40, 0, 0, 0, 0, 0]);
        assert_eq!(lun_address(3, 0x123), [1, 3, 0x41, 0x23, 0, 0, 0, 0]);
    }

    #[test]
    fn header_offsets() {
        let r = ReqCmd::zeroed();
        let base = &r as *const _ as usize;
        assert_eq!(core::ptr::addr_of!(r.id) as usize - base, 8);
        assert_eq!(core::ptr::addr_of!(r.cdb) as usize - base, 19);
        let s = RespCmd::zeroed();
        let base = &s as *const _ as usize;
        assert_eq!(core::ptr::addr_of!(s.status) as usize - base, 10);
        assert_eq!(core::ptr::addr_of!(s.response) as usize - base, 11);
        assert_eq!(core::ptr::addr_of!(s.sense) as usize - base, 12);
    }
}

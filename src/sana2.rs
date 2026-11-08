//! SANA-II network devices (devices/sana2.h, SANA-II Revision 7): the
//! request and statistics layouts, the commands, errors and events, and,
//! for a driver, the buffer management a protocol stack hands it.
//!
//! - [`IOSana2Req`] and [`S2Req`]: the request, and its fields by name.
//! - [`BufferManagement`]: the stack's copy functions, from OpenDevice's
//!   tags or, once the stack sends `S2_SANA2HOOK`, its [`Sana2Hook`]. The
//!   hook is called through `utility.library/CallHookPkt` (native code).
//!   The tag functions are 68k-convention code (A0 destination, A1 source,
//!   D0 length) and are called through `exec.library/Emulate`, unless
//!   `IsNative` says they are native, when they are called as
//!   `fn(to, from, len) -> u32`.
//!
//! Layouts are the SDK's (pack 2 on PPC); their sizes and offsets are
//! checked at compile time, and `tests/sana2_consts.rs` checks the
//! constants against the SDK.

use core::ffi::c_void;
use core::ptr::{addr_of, addr_of_mut, null_mut};

use crate::sys::{self, IORequest, Tag, TagItem, TAG_DONE, TAG_IGNORE, TAG_MORE, TAG_SKIP};

pub const SANA2_MAX_ADDR_BITS: usize = 128;
pub const SANA2_MAX_ADDR_BYTES: usize = (SANA2_MAX_ADDR_BITS + 7) / 8;

// ---- Layouts (pack 2) ----

/// `struct MinNode` (utility/hooks.h's `struct Hook` starts with one).
#[repr(C, packed(2))]
#[derive(Clone, Copy)]
pub struct MinNode {
    pub mln_Succ: *mut MinNode,
    pub mln_Pred: *mut MinNode,
}

/// The function a [`Hook`] calls: `(hook, object, message) -> u32`.
pub type HookFunc = unsafe extern "C" fn(*mut Hook, *mut c_void, *mut c_void) -> u32;

/// `struct Hook` (utility/hooks.h).
#[repr(C, packed(2))]
pub struct Hook {
    pub h_MinNode: MinNode,
    pub h_Entry: Option<HookFunc>,
    pub h_SubEntry: Option<HookFunc>,
    pub h_Data: *mut c_void,
}

/// `struct IOSana2Req`.
#[repr(C, packed(2))]
pub struct IOSana2Req {
    /// struct IORequest (32 bytes): Message, io_Device, io_Unit,
    /// io_Command, io_Flags, io_Error.
    pub ios2_Req: [u8; 32],
    pub ios2_WireError: u32,
    pub ios2_PacketType: u32,
    pub ios2_SrcAddr: [u8; SANA2_MAX_ADDR_BYTES],
    pub ios2_DstAddr: [u8; SANA2_MAX_ADDR_BYTES],
    pub ios2_DataLength: u32,
    pub ios2_Data: *mut c_void,
    pub ios2_StatData: *mut c_void,
    pub ios2_BufferManagement: *mut c_void,
}

/// `struct Sana2DeviceQuery`. A driver fills only as much as
/// `SizeAvailable` allows (some old drivers fail if it exceeds 30 bytes,
/// which ends before `RawMTU`).
#[repr(C, packed(2))]
pub struct Sana2DeviceQuery {
    pub SizeAvailable: u32,
    pub SizeSupplied: u32,
    pub DevQueryFormat: u32,
    pub DeviceLevel: u32,
    pub AddrFieldSize: u16,
    pub MTU: u32,
    pub BPS: u32,
    pub HardwareType: u32,
    pub RawMTU: u32,
}

/// `struct Sana2PacketTypeStats` (S2_GETTYPESTATS).
#[repr(C, packed(2))]
#[derive(Clone, Copy, Default)]
pub struct Sana2PacketTypeStats {
    pub PacketsSent: u32,
    pub PacketsReceived: u32,
    pub BytesSent: u32,
    pub BytesReceived: u32,
    pub PacketsDropped: u32,
}

/// `struct Sana2SpecialStatRecord`.
#[repr(C, packed(2))]
pub struct Sana2SpecialStatRecord {
    pub Type: u32,
    pub Count: u32,
    pub String: *const u8,
}

/// `struct Sana2SpecialStatHeader`, followed by RecordCountMax records.
#[repr(C, packed(2))]
pub struct Sana2SpecialStatHeader {
    pub RecordCountMax: u32,
    pub RecordCountSupplied: u32,
}

/// `struct TimeVal` (devices/timer.h).
#[repr(C, packed(2))]
#[derive(Clone, Copy, Default)]
pub struct TimeVal {
    pub Seconds: u32,
    pub Microseconds: u32,
}

/// `struct Sana2DeviceStats` (S2_GETGLOBALSTATS).
#[repr(C, packed(2))]
pub struct Sana2DeviceStats {
    pub PacketsReceived: u32,
    pub PacketsSent: u32,
    pub BadData: u32,
    pub Overruns: u32,
    pub Unused: u32,
    pub UnknownTypesReceived: u32,
    pub Reconfigurations: u32,
    pub LastStart: TimeVal,
}

/// `S2QUAD`: a 64-bit big-endian counter as two words.
#[repr(C, packed(2))]
#[derive(Clone, Copy, Default)]
pub struct S2Quad {
    pub s2q_High: u32,
    pub s2q_Low: u32,
}

impl From<u64> for S2Quad {
    fn from(v: u64) -> Self {
        S2Quad { s2q_High: (v >> 32) as u32, s2q_Low: v as u32 }
    }
}

/// `struct Sana2ExtDeviceStats` (S2_GETEXTENDEDGLOBALSTATS).
#[repr(C, packed(2))]
pub struct Sana2ExtDeviceStats {
    pub s2xds_Length: u32,
    pub s2xds_Actual: u32,
    pub s2xds_PacketsReceived: S2Quad,
    pub s2xds_PacketsSent: S2Quad,
    pub s2xds_BadData: S2Quad,
    pub s2xds_Overruns: S2Quad,
    pub s2xds_UnknownTypesReceived: S2Quad,
    pub s2xds_Reconfigurations: S2Quad,
    pub s2xds_LastStart: TimeVal,
    pub s2xds_LastConnected: TimeVal,
    pub s2xds_LastDisconnected: TimeVal,
    pub s2xds_TimeConnected: TimeVal,
}

/// `struct Sana2ThroughputStats` (S2_SAMPLE_THROUGHPUT).
#[repr(C, packed(2))]
pub struct Sana2ThroughputStats {
    pub s2ts_Length: u32,
    pub s2ts_Actual: u32,
    pub s2ts_NotifyTask: *mut sys::Task,
    pub s2ts_NotifyMask: u32,
    pub s2ts_StartTime: TimeVal,
    pub s2ts_EndTime: TimeVal,
    pub s2ts_BytesSent: S2Quad,
    pub s2ts_BytesReceived: S2Quad,
    pub s2ts_Updates: S2Quad,
}

/// `struct Sana2Hook` (S2_SANA2HOOK): the hook and the methods it takes,
/// a `TAG_END`-terminated list of method tags.
#[repr(C, packed(2))]
pub struct Sana2Hook {
    pub s2h_Hook: Hook,
    pub s2h_Methods: *const Tag,
}

/// `struct SANA2HookMsg`: the start of every hook message.
#[repr(C, packed(2))]
pub struct SANA2HookMsg {
    pub shm_Method: u32,
    pub shm_MsgSize: u32,
}

/// `struct SANA2CopyHookMsg`: the copy and DMA methods' message.
#[repr(C, packed(2))]
pub struct SANA2CopyHookMsg {
    pub schm_Method: u32,
    pub schm_MsgSize: u32,
    pub schm_To: *mut c_void,
    pub schm_From: *mut c_void,
    pub schm_Size: u32,
}

/// `struct SANA2LogHookMsg`: the S2_Log method's message.
#[repr(C, packed(2))]
pub struct SANA2LogHookMsg {
    pub slhm_Method: u32,
    pub slhm_MsgSize: u32,
    pub slhm_Priority: u32,
    pub slhm_Name: *const u8,
    pub slhm_Message: *const u8,
}

// Pointer-sized fields: the layouts are the target's (32-bit pointers).
#[cfg(target_pointer_width = "32")]
const _: () = {
    use core::mem::{offset_of, size_of};
    assert!(size_of::<MinNode>() == 8);
    assert!(size_of::<Hook>() == 20);
    assert!(size_of::<IOSana2Req>() == 88);
    assert!(offset_of!(IOSana2Req, ios2_WireError) == 32);
    assert!(offset_of!(IOSana2Req, ios2_PacketType) == 36);
    assert!(offset_of!(IOSana2Req, ios2_SrcAddr) == 40);
    assert!(offset_of!(IOSana2Req, ios2_DstAddr) == 56);
    assert!(offset_of!(IOSana2Req, ios2_DataLength) == 72);
    assert!(offset_of!(IOSana2Req, ios2_Data) == 76);
    assert!(offset_of!(IOSana2Req, ios2_StatData) == 80);
    assert!(offset_of!(IOSana2Req, ios2_BufferManagement) == 84);
    assert!(size_of::<Sana2DeviceQuery>() == 34);
    assert!(offset_of!(Sana2DeviceQuery, AddrFieldSize) == 16);
    assert!(offset_of!(Sana2DeviceQuery, MTU) == 18);
    assert!(offset_of!(Sana2DeviceQuery, HardwareType) == 26);
    assert!(offset_of!(Sana2DeviceQuery, RawMTU) == 30);
    assert!(size_of::<Sana2PacketTypeStats>() == 20);
    assert!(size_of::<Sana2SpecialStatRecord>() == 12);
    assert!(size_of::<Sana2DeviceStats>() == 36);
    assert!(size_of::<Sana2ExtDeviceStats>() == 88);
    assert!(size_of::<Sana2ThroughputStats>() == 56);
    assert!(offset_of!(Sana2ThroughputStats, s2ts_BytesSent) == 32);
    assert!(size_of::<Sana2Hook>() == 24);
    assert!(size_of::<SANA2CopyHookMsg>() == 20);
    assert!(size_of::<SANA2LogHookMsg>() == 20);
};

// ---- io_Flags and OpenDevice flags ----

pub const SANA2IOB_RAW: u8 = 7;
pub const SANA2IOB_BCAST: u8 = 6;
pub const SANA2IOB_MCAST: u8 = 5;
pub const SANA2IOB_QUICK: u8 = 0;
pub const SANA2IOF_RAW: u8 = 1 << SANA2IOB_RAW;
pub const SANA2IOF_BCAST: u8 = 1 << SANA2IOB_BCAST;
pub const SANA2IOF_MCAST: u8 = 1 << SANA2IOB_MCAST;
pub const SANA2IOF_QUICK: u8 = 1 << SANA2IOB_QUICK;

pub const SANA2OPB_MINE: u32 = 0;
pub const SANA2OPB_PROM: u32 = 1;
pub const SANA2OPF_MINE: u32 = 1 << SANA2OPB_MINE;
pub const SANA2OPF_PROM: u32 = 1 << SANA2OPB_PROM;

// ---- OpenDevice tags and hook methods ----

pub const S2_DUMMY: Tag = sys::TAG_USER + 0xB0000;
pub const S2_COPY_TO_BUFF: Tag = S2_DUMMY + 1;
pub const S2_COPY_FROM_BUFF: Tag = S2_DUMMY + 2;
pub const S2_PACKET_FILTER: Tag = S2_DUMMY + 3;
pub const S2_COPY_TO_BUFF16: Tag = S2_DUMMY + 4;
pub const S2_COPY_FROM_BUFF16: Tag = S2_DUMMY + 5;
pub const S2_COPY_TO_BUFF32: Tag = S2_DUMMY + 6;
pub const S2_COPY_FROM_BUFF32: Tag = S2_DUMMY + 7;
pub const S2_DMA_COPY_TO_BUFF32: Tag = S2_DUMMY + 8;
pub const S2_DMA_COPY_FROM_BUFF32: Tag = S2_DUMMY + 9;
pub const S2_DMA_COPY_TO_BUFF64: Tag = S2_DUMMY + 10;
pub const S2_DMA_COPY_FROM_BUFF64: Tag = S2_DUMMY + 11;
pub const S2_LOG: Tag = S2_DUMMY + 12;

// ---- Hardware types ----

pub const S2_WIRETYPE_ETHERNET: u32 = 1;
pub const S2_WIRETYPE_IEEE802: u32 = 6;
pub const S2_WIRETYPE_ARCNET: u32 = 7;
pub const S2_WIRETYPE_LOCALTALK: u32 = 11;
pub const S2_WIRETYPE_DYLAN: u32 = 12;
pub const S2_WIRETYPE_AMOKNET: u32 = 200;
pub const S2_WIRETYPE_LIANA: u32 = 202;
pub const S2_WIRETYPE_PPP: u32 = 253;
pub const S2_WIRETYPE_SLIP: u32 = 254;
pub const S2_WIRETYPE_CSLIP: u32 = 255;
pub const S2_WIRETYPE_PLIP: u32 = 420;

// ---- Log levels ----

pub const S2LOG_EMERGENCY: u32 = 0;
pub const S2LOG_ALERT: u32 = 1;
pub const S2LOG_CRITICAL: u32 = 2;
pub const S2LOG_ERROR: u32 = 3;
pub const S2LOG_WARNING: u32 = 4;
pub const S2LOG_NOTICE: u32 = 5;
pub const S2LOG_INFORMATION: u32 = 6;
pub const S2LOG_DEBUG: u32 = 7;

// ---- Commands ----

pub const S2_START: u16 = crate::device::cmd::CMD_NONSTD;
pub const S2_DEVICEQUERY: u16 = S2_START;
pub const S2_GETSTATIONADDRESS: u16 = S2_START + 1;
pub const S2_CONFIGINTERFACE: u16 = S2_START + 2;
pub const S2_ADDMULTICASTADDRESS: u16 = S2_START + 5;
pub const S2_DELMULTICASTADDRESS: u16 = S2_START + 6;
pub const S2_MULTICAST: u16 = S2_START + 7;
pub const S2_BROADCAST: u16 = S2_START + 8;
pub const S2_TRACKTYPE: u16 = S2_START + 9;
pub const S2_UNTRACKTYPE: u16 = S2_START + 10;
pub const S2_GETTYPESTATS: u16 = S2_START + 11;
pub const S2_GETSPECIALSTATS: u16 = S2_START + 12;
pub const S2_GETGLOBALSTATS: u16 = S2_START + 13;
pub const S2_ONEVENT: u16 = S2_START + 14;
pub const S2_READORPHAN: u16 = S2_START + 15;
pub const S2_ONLINE: u16 = S2_START + 16;
pub const S2_OFFLINE: u16 = S2_START + 17;
pub const S2_END: u16 = S2_START + 18;
pub const S2_ADDMULTICASTADDRESSES: u16 = 0xC000;
pub const S2_DELMULTICASTADDRESSES: u16 = 0xC001;
pub const S2_GETPEERADDRESS: u16 = 0xC002;
pub const S2_GETDNSADDRESS: u16 = 0xC003;
pub const S2_GETEXTENDEDGLOBALSTATS: u16 = 0xC004;
pub const S2_CONNECT: u16 = 0xC005;
pub const S2_DISCONNECT: u16 = 0xC006;
pub const S2_SAMPLE_THROUGHPUT: u16 = 0xC007;
pub const S2_SANA2HOOK: u16 = 0xC008;

/// devices/newstyle.h: NSCMD_DEVICEQUERY's type for a SANA-II device.
pub const NSDEVTYPE_SANA2: u16 = 7;

// ---- io_Error ----

pub const S2ERR_NO_ERROR: i8 = 0;
pub const S2ERR_NO_RESOURCES: i8 = 1;
pub const S2ERR_BAD_ARGUMENT: i8 = 3;
pub const S2ERR_BAD_STATE: i8 = 4;
pub const S2ERR_BAD_ADDRESS: i8 = 5;
pub const S2ERR_MTU_EXCEEDED: i8 = 6;
pub const S2ERR_NOT_SUPPORTED: i8 = 8;
pub const S2ERR_SOFTWARE: i8 = 9;
pub const S2ERR_OUTOFSERVICE: i8 = 10;
pub const S2ERR_TX_FAILURE: i8 = 11;

// ---- ios2_WireError ----

pub const S2WERR_GENERIC_ERROR: u32 = 0;
pub const S2WERR_NOT_CONFIGURED: u32 = 1;
pub const S2WERR_UNIT_ONLINE: u32 = 2;
pub const S2WERR_UNIT_OFFLINE: u32 = 3;
pub const S2WERR_ALREADY_TRACKED: u32 = 4;
pub const S2WERR_NOT_TRACKED: u32 = 5;
pub const S2WERR_BUFF_ERROR: u32 = 6;
pub const S2WERR_SRC_ADDRESS: u32 = 7;
pub const S2WERR_DST_ADDRESS: u32 = 8;
pub const S2WERR_BAD_BROADCAST: u32 = 9;
pub const S2WERR_BAD_MULTICAST: u32 = 10;
pub const S2WERR_MULTICAST_FULL: u32 = 11;
pub const S2WERR_BAD_EVENT: u32 = 12;
pub const S2WERR_BAD_STATDATA: u32 = 13;
pub const S2WERR_IS_CONFIGURED: u32 = 15;
pub const S2WERR_NULL_POINTER: u32 = 16;
pub const S2WERR_TOO_MANY_RETRIES: u32 = 17;
pub const S2WERR_RCVREL_HDW_ERR: u32 = 18;
pub const S2WERR_UNIT_DISCONNECTED: u32 = 19;
pub const S2WERR_UNIT_CONNECTED: u32 = 20;
pub const S2WERR_INVALID_OPTION: u32 = 21;
pub const S2WERR_MISSING_OPTION: u32 = 22;
pub const S2WERR_AUTHENTICATION_FAILED: u32 = 23;
pub const S2WERR_FUNCTIONS_MISSING: u32 = 24;

// ---- Events ----

pub const S2EVENT_ERROR: u32 = 1 << 0;
pub const S2EVENT_TX: u32 = 1 << 1;
pub const S2EVENT_RX: u32 = 1 << 2;
pub const S2EVENT_ONLINE: u32 = 1 << 3;
pub const S2EVENT_OFFLINE: u32 = 1 << 4;
pub const S2EVENT_BUFF: u32 = 1 << 5;
pub const S2EVENT_HARDWARE: u32 = 1 << 6;
pub const S2EVENT_SOFTWARE: u32 = 1 << 7;
pub const S2EVENT_CONFIGCHANGED: u32 = 1 << 8;
pub const S2EVENT_CONNECT: u32 = 1 << 9;
pub const S2EVENT_DISCONNECT: u32 = 1 << 10;

// ---- A request, by name ----

/// A caller's `IOSana2Req`, by pointer (Copy: it names the request).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct S2Req(*mut IOSana2Req);

// SAFETY: requests move between tasks by exec message passing.
unsafe impl Send for S2Req {}

macro_rules! s2field {
    ($get:ident, $set:ident, $f:ident, $t:ty) => {
        #[inline]
        pub fn $get(self) -> $t {
            // SAFETY: from_raw's contract.
            unsafe { addr_of!((*self.0).$f).read_unaligned() }
        }
        #[inline]
        // `v` is stored, never dereferenced (it may be a pointer field).
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        pub fn $set(self, v: $t) {
            // SAFETY: from_raw's contract.
            unsafe { addr_of_mut!((*self.0).$f).write_unaligned(v) }
        }
    };
}

impl S2Req {
    /// # Safety
    /// `ior` is a live `IOSana2Req` (88 bytes) while this value is used.
    #[inline]
    pub unsafe fn from_raw(ior: *mut IORequest) -> Option<Self> {
        if ior.is_null() { None } else { Some(Self(ior as *mut IOSana2Req)) }
    }
    #[inline]
    pub fn as_ptr(self) -> *mut IORequest {
        self.0 as *mut IORequest
    }
    #[inline]
    pub fn raw(self) -> *mut IOSana2Req {
        self.0
    }
    /// The request as an exec IORequest, for the generic fields and replies.
    #[inline]
    pub fn io(self) -> crate::device::IoReq {
        // SAFETY: an IOSana2Req starts with its IORequest (an IOStdReq-sized
        // block is not required for the fields IoReq reads below 32).
        unsafe { crate::device::IoReq::from_raw(self.0 as *mut IORequest) }.expect("non-null")
    }
    s2field!(wire_error, set_wire_error, ios2_WireError, u32);
    s2field!(packet_type, set_packet_type, ios2_PacketType, u32);
    s2field!(src_addr, set_src_addr, ios2_SrcAddr, [u8; SANA2_MAX_ADDR_BYTES]);
    s2field!(dst_addr, set_dst_addr, ios2_DstAddr, [u8; SANA2_MAX_ADDR_BYTES]);
    s2field!(data_length, set_data_length, ios2_DataLength, u32);
    s2field!(data, set_data, ios2_Data, *mut c_void);
    s2field!(stat_data, set_stat_data, ios2_StatData, *mut c_void);
    s2field!(buffer_management, set_buffer_management, ios2_BufferManagement, *mut c_void);

    /// The first six bytes of ios2_SrcAddr / ios2_DstAddr (Ethernet).
    pub fn src_mac(self) -> [u8; 6] {
        let a = self.src_addr();
        [a[0], a[1], a[2], a[3], a[4], a[5]]
    }
    pub fn dst_mac(self) -> [u8; 6] {
        let a = self.dst_addr();
        [a[0], a[1], a[2], a[3], a[4], a[5]]
    }
    pub fn set_src_mac(self, m: &[u8; 6]) {
        let mut a = [0u8; SANA2_MAX_ADDR_BYTES];
        a[..6].copy_from_slice(m);
        self.set_src_addr(a);
    }
    pub fn set_dst_mac(self, m: &[u8; 6]) {
        let mut a = [0u8; SANA2_MAX_ADDR_BYTES];
        a[..6].copy_from_slice(m);
        self.set_dst_addr(a);
    }

    /// End the request with io_Error and ios2_WireError (io_Actual is not
    /// a SANA-II field and is left alone), replying unless quick.
    ///
    /// # Safety
    /// As for `IoReq::finish`.
    pub unsafe fn finish(self, error: i8, wire: u32) {
        self.set_wire_error(wire);
        let r = self.io();
        r.set_error(error);
        if !r.is_quick() {
            // SAFETY: a live request that is ours to end (as for IoReq::finish).
            unsafe { sys::exec_reply_msg(self.0 as *mut sys::Message) };
        }
    }

    /// End a queued (never quick) request with io_Error and ios2_WireError.
    ///
    /// # Safety
    /// As for `IoReq::reply`.
    pub unsafe fn reply(self, error: i8, wire: u32) {
        self.set_wire_error(wire);
        self.io().set_error(error);
        // SAFETY: a live request that is ours to end (as for IoReq::reply).
        unsafe { sys::exec_reply_msg(self.0 as *mut sys::Message) };
    }
}

// ---- Buffer management, for a driver ----

/// exec/emulation.h: Emulate()'s register tags.
const ET_REGISTER_D0: Tag = sys::TAG_USER + 1;
const ET_REGISTER_A0: Tag = sys::TAG_USER + 9;
const ET_REGISTER_A1: Tag = sys::TAG_USER + 10;

/// What a stack gave a driver to move packet data in and out of its own
/// buffers: the OpenDevice tag functions, and, after `S2_SANA2HOOK`, the
/// hook that replaces them. A driver keeps one per opener and returns its
/// address in `ios2_BufferManagement` (the cookie every later request
/// carries).
pub struct BufferManagement {
    copy_to: *const c_void,
    copy_from: *const c_void,
    filter: *mut Hook,
    hook: *mut Sana2Hook,
    hook_dma_to: bool,
    hook_dma_from: bool,
}

/// Why `S2_SANA2HOOK` was refused: io_Error and ios2_WireError.
pub type HookRefusal = (i8, u32);

impl BufferManagement {
    /// From OpenDevice's tag list (S2_CopyToBuff, S2_CopyFromBuff,
    /// S2_PacketFilter); TAG_IGNORE, TAG_MORE and TAG_SKIP are followed.
    /// A stack that gives only the 16- or 32-bit variants (S2_CopyToBuff16,
    /// S2_CopyFromBuff16/32, same calling convention) gets those used.
    ///
    /// # Safety
    /// `tags` is null or a valid tag list.
    pub unsafe fn from_tags(tags: *const TagItem) -> Self {
        let mut bm = BufferManagement {
            copy_to: core::ptr::null(),
            copy_from: core::ptr::null(),
            filter: null_mut(),
            hook: null_mut(),
            hook_dma_to: false,
            hook_dma_from: false,
        };
        let mut alt_to: *const c_void = core::ptr::null();
        let mut alt_from: *const c_void = core::ptr::null();
        // SAFETY: `tags` is null or a valid tag list (the contract), and
        // TAG_MORE/TAG_SKIP stay within the lists it links.
        unsafe {
            let mut t = tags;
            let mut guard = 0;
            while !t.is_null() && guard < 1000 {
                guard += 1;
                let TagItem { ti_Tag, ti_Data } = core::ptr::read_unaligned(t);
                match ti_Tag {
                    TAG_DONE => break,
                    TAG_IGNORE => t = t.add(1),
                    TAG_MORE => t = ti_Data as usize as *const TagItem,
                    TAG_SKIP => t = t.add(1 + ti_Data as usize),
                    S2_COPY_TO_BUFF => {
                        bm.copy_to = ti_Data as usize as *const c_void;
                        t = t.add(1)
                    }
                    S2_COPY_FROM_BUFF => {
                        bm.copy_from = ti_Data as usize as *const c_void;
                        t = t.add(1)
                    }
                    S2_PACKET_FILTER => {
                        bm.filter = ti_Data as usize as *mut Hook;
                        t = t.add(1)
                    }
                    S2_COPY_TO_BUFF16 => {
                        alt_to = ti_Data as usize as *const c_void;
                        t = t.add(1)
                    }
                    S2_COPY_FROM_BUFF16 | S2_COPY_FROM_BUFF32 => {
                        if alt_from.is_null() {
                            alt_from = ti_Data as usize as *const c_void;
                        }
                        t = t.add(1)
                    }
                    _ => t = t.add(1),
                }
            }
        }
        if bm.copy_to.is_null() {
            bm.copy_to = alt_to;
        }
        if bm.copy_from.is_null() {
            bm.copy_from = alt_from;
        }
        bm
    }

    /// Both mandatory functions are there, as tags or through the hook.
    pub fn complete(&self) -> bool {
        !self.hook.is_null() || (!self.copy_to.is_null() && !self.copy_from.is_null())
    }

    /// The stack installed its hook.
    pub fn has_hook(&self) -> bool {
        !self.hook.is_null()
    }

    /// `S2_SANA2HOOK`: check and install the stack's hook, which replaces
    /// the tag functions until CloseDevice. `len` is ios2_DataLength.
    ///
    /// # Safety
    /// `hook` is null or a Sana2Hook valid until CloseDevice.
    pub unsafe fn install_hook(&mut self, hook: *mut Sana2Hook, len: u32) -> Result<(), HookRefusal> {
        if hook.is_null() {
            return Err((S2ERR_BAD_ARGUMENT, S2WERR_NULL_POINTER));
        }
        if len < 20 {
            return Err((crate::device::ioerr::IOERR_BADLENGTH, S2WERR_GENERIC_ERROR));
        }
        if !self.hook.is_null() {
            return Err((crate::device::ioerr::IOERR_UNITBUSY, S2WERR_GENERIC_ERROR));
        }
        // SAFETY: `hook` is a valid Sana2Hook (the contract).
        let methods = unsafe { addr_of!((*hook).s2h_Methods).read_unaligned() };
        let has = |m: Tag| -> bool {
            let mut p = methods;
            let mut guard = 0;
            while !p.is_null() && guard < 64 {
                // SAFETY: s2h_Methods is a TAG_DONE-ended array (the
                // hook's contract); the walk stops at TAG_DONE or 64.
                let t = unsafe { core::ptr::read_unaligned(p) };
                if t == TAG_DONE {
                    return false;
                }
                if t == m {
                    return true;
                }
                // SAFETY: still inside the array (not past TAG_DONE).
                p = unsafe { p.add(1) };
                guard += 1;
            }
            false
        };
        if !has(S2_COPY_TO_BUFF) || !has(S2_COPY_FROM_BUFF) {
            return Err((S2ERR_BAD_ARGUMENT, S2WERR_FUNCTIONS_MISSING));
        }
        self.hook_dma_to = has(S2_DMA_COPY_TO_BUFF32);
        self.hook_dma_from = has(S2_DMA_COPY_FROM_BUFF32);
        self.hook = hook;
        Ok(())
    }

    /// # Safety
    /// The hook is installed (non-null), and `to`/`from`/`n` are what
    /// `method` documents.
    unsafe fn call_hook(&self, ior: S2Req, method: Tag, to: *mut c_void, from: *mut c_void, n: u32) -> u32 {
        let mut msg = SANA2CopyHookMsg {
            schm_Method: method,
            schm_MsgSize: core::mem::size_of::<SANA2CopyHookMsg>() as u32,
            schm_To: to,
            schm_From: from,
            schm_Size: n,
        };
        // SAFETY: the installed hook (the contract), a live request, and a
        // message on this stack.
        unsafe {
            sys::utility_call_hook_pkt(
                addr_of_mut!((*self.hook).s2h_Hook) as *mut sys::Hook,
                ior.raw() as *mut c_void,
                &mut msg as *mut SANA2CopyHookMsg as *mut c_void,
            )
        }
    }

    /// # Safety
    /// `f` is null or the stack's copy function (native or 68k), and
    /// `to`/`from`/`n` are what it documents.
    unsafe fn call_fn(f: *const c_void, to: *mut c_void, from: *mut c_void, n: u32) -> u32 {
        if f.is_null() {
            return 0;
        }
        // SAFETY: IsNative only inspects the address.
        if unsafe { sys::exec_is_native(f) } != 0 {
            // SAFETY: a native copy function with this signature (the
            // SANA-II convention for S2_CopyToBuff/S2_CopyFromBuff).
            unsafe {
                let native: unsafe extern "C" fn(*mut c_void, *mut c_void, u32) -> u32 = core::mem::transmute(f);
                native(to, from, n)
            }
        } else {
            let tags = [
                TagItem { ti_Tag: ET_REGISTER_A0, ti_Data: to as usize as u32 },
                TagItem { ti_Tag: ET_REGISTER_A1, ti_Data: from as usize as u32 },
                TagItem { ti_Tag: ET_REGISTER_D0, ti_Data: n },
                TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
            ];
            // SAFETY: 68k code with its registers given as tags.
            unsafe { sys::exec_emulate(f, tags.as_ptr()) }
        }
    }

    /// Copy `n` bytes from the driver's `from` into the stack's buffer
    /// `to_cookie` (a CMD_READ's ios2_Data). False: the stack refused.
    ///
    /// # Safety
    /// `from` holds `n` bytes; the request and cookie are the stack's.
    pub unsafe fn copy_to_buff(&self, ior: S2Req, to_cookie: *mut c_void, from: *const u8, n: u32) -> bool {
        // SAFETY: `from` holds `n` bytes and the request and cookie are the
        // stack's (the contract); the hook or function is the stack's.
        unsafe {
            if !self.hook.is_null() {
                self.call_hook(ior, S2_COPY_TO_BUFF, to_cookie, from as *mut c_void, n) != 0
            } else {
                Self::call_fn(self.copy_to, to_cookie, from as *mut c_void, n) != 0
            }
        }
    }

    /// Copy `n` bytes from the stack's buffer `from_cookie` (a CMD_WRITE's
    /// ios2_Data) into the driver's contiguous `to`. False: refused.
    ///
    /// # Safety
    /// `to` has room for `n` bytes; the request and cookie are the stack's.
    pub unsafe fn copy_from_buff(&self, ior: S2Req, to: *mut u8, from_cookie: *mut c_void, n: u32) -> bool {
        // SAFETY: `to` has room for `n` bytes and the request and cookie
        // are the stack's (the contract).
        unsafe {
            if !self.hook.is_null() {
                self.call_hook(ior, S2_COPY_FROM_BUFF, to as *mut c_void, from_cookie, n) != 0
            } else {
                Self::call_fn(self.copy_from, to as *mut c_void, from_cookie, n) != 0
            }
        }
    }

    /// The stack's packet filter (S2_PacketFilter), asked before a packet is
    /// copied to a CMD_READ: true to deliver it. No filter: true. `ior`'s
    /// ios2_DataLength, ios2_SrcAddr and ios2_DstAddr must be set first.
    ///
    /// # Safety
    /// `data` is the packet data the filter may read.
    pub unsafe fn filter(&self, ior: S2Req, data: *const u8) -> bool {
        if self.filter.is_null() {
            return true;
        }
        // SAFETY: the stack's filter hook, a live request, and the packet
        // data (the contract).
        unsafe { sys::utility_call_hook_pkt(self.filter as *mut sys::Hook, ior.raw() as *mut c_void, data as *mut c_void) != 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_tags_follows_skip_and_ignore() {
        let tags = [
            TagItem { ti_Tag: TAG_IGNORE, ti_Data: 0 },
            TagItem { ti_Tag: TAG_SKIP, ti_Data: 1 },
            TagItem { ti_Tag: S2_COPY_TO_BUFF, ti_Data: 0xdead }, // skipped
            TagItem { ti_Tag: S2_COPY_TO_BUFF, ti_Data: 0x1000 },
            TagItem { ti_Tag: S2_PACKET_FILTER, ti_Data: 0x3000 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
            TagItem { ti_Tag: S2_COPY_FROM_BUFF, ti_Data: 0x2000 }, // after the end
        ];
        // SAFETY: a TAG_DONE-ended list.
        let bm = unsafe { BufferManagement::from_tags(tags.as_ptr()) };
        assert_eq!(bm.copy_to as usize, 0x1000);
        assert_eq!(bm.filter as usize, 0x3000);
        assert!(bm.copy_from.is_null() && !bm.complete());
    }

    // TAG_MORE carries a 32-bit pointer: only meaningful on the target.
    #[cfg(target_pointer_width = "32")]
    #[test]
    fn from_tags_follows_more() {
        let tail = [
            TagItem { ti_Tag: S2_COPY_FROM_BUFF, ti_Data: 0x2000 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        let head = [
            TagItem { ti_Tag: S2_COPY_TO_BUFF, ti_Data: 0x1000 },
            TagItem { ti_Tag: TAG_MORE, ti_Data: tail.as_ptr() as usize as u32 },
        ];
        // SAFETY: a list whose TAG_MORE links a TAG_DONE-ended one.
        assert!(unsafe { BufferManagement::from_tags(head.as_ptr()) }.complete());
    }

    #[test]
    fn hook_install_checks_methods_and_length() {
        static METHODS_OK: [Tag; 3] = [S2_COPY_TO_BUFF, S2_COPY_FROM_BUFF, TAG_DONE];
        static METHODS_SHORT: [Tag; 2] = [S2_COPY_TO_BUFF, TAG_DONE];
        let mk = |m: &'static [Tag]| Sana2Hook {
            s2h_Hook: Hook {
                h_MinNode: MinNode { mln_Succ: null_mut(), mln_Pred: null_mut() },
                h_Entry: None,
                h_SubEntry: None,
                h_Data: null_mut(),
            },
            s2h_Methods: m.as_ptr(),
        };
        // SAFETY: null is allowed.
        let mut bm = unsafe { BufferManagement::from_tags(core::ptr::null()) };
        let mut short = mk(&METHODS_SHORT);
        // SAFETY: (this and the calls below) hooks on this stack that
        // outlive `bm`'s use, with TAG_DONE-ended static method lists.
        assert_eq!(unsafe { bm.install_hook(&mut short, 24) }, Err((S2ERR_BAD_ARGUMENT, S2WERR_FUNCTIONS_MISSING)));
        let mut ok = mk(&METHODS_OK);
        // SAFETY: as above.
        assert_eq!(unsafe { bm.install_hook(&mut ok, 10) }.unwrap_err().0, crate::device::ioerr::IOERR_BADLENGTH);
        // SAFETY: as above.
        assert!(unsafe { bm.install_hook(&mut ok, 24) }.is_ok());
        assert!(bm.complete() && bm.has_hook());
        // SAFETY: as above.
        assert_eq!(unsafe { bm.install_hook(&mut ok, 24) }.unwrap_err().0, crate::device::ioerr::IOERR_UNITBUSY);
    }

    #[test]
    fn s2quad_splits_words() {
        let q = S2Quad::from(0x0000_0001_0000_0002u64);
        let (h, l) = (q.s2q_High, q.s2q_Low);
        assert_eq!((h, l), (1, 2));
    }
}

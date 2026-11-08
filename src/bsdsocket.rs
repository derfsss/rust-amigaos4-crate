//! bsdsocket.library (Roadshow) for programs: the library, IPv4 sockets,
//! interface queries and the stack's statistics.
//!
//! - [`SocketLib`]: opens `bsdsocket.library` and its `main` interface
//!   ([`SocketIFace`](crate::sys::SocketIFace)), and wraps the calls a
//!   program needs: sockets, `QueryInterfaceTagList`,
//!   `ObtainInterfaceList`, `GetNetworkStatistics`, `SocketBaseTagList`.
//! - The statistics blocks `GetNetworkStatistics` fills are arrays of
//!   32-bit counters; [`TCPSTAT_FIELDS`], [`IPSTAT_FIELDS`],
//!   [`UDPSTAT_FIELDS`] and [`MBSTAT_FIELDS`] name them in order.
//!
//! Constants keep the SDK's names (netinclude), and `tests/bsdsocket_consts.rs`
//! checks them and the field tables against the SDK headers. Each opener
//! gets its own library base; a socket belongs to the base that made it.

#![allow(non_upper_case_globals, non_snake_case)]

use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::ptr::null_mut;

use crate::error::{AmigaError, Result};
use crate::sys::{self, Interface, Library, Node, SocketIFace, Tag, TagItem, TAG_DONE, TAG_USER};

// ---- sys/socket.h, netinet/in.h, netinet/tcp.h, sys/errno.h ----

pub const AF_INET: i32 = 2;
pub const SOCK_STREAM: i32 = 1;
pub const SOCK_DGRAM: i32 = 2;
pub const SOL_SOCKET: i32 = 0xffff;
pub const SO_REUSEADDR: i32 = 0x0004;
pub const SO_SNDBUF: i32 = 0x1001;
pub const SO_RCVBUF: i32 = 0x1002;
pub const SO_SNDTIMEO: i32 = 0x1005;
pub const SO_RCVTIMEO: i32 = 0x1006;
pub const MSG_PEEK: i32 = 0x2;
pub const MSG_WAITALL: i32 = 0x40;
pub const MSG_DONTWAIT: i32 = 0x80;
pub const IPPROTO_IP: i32 = 0;
pub const IPPROTO_TCP: i32 = 6;
pub const IPPROTO_UDP: i32 = 17;
pub const INADDR_ANY: u32 = 0;
pub const TCP_NODELAY: i32 = 0x01;
pub const TCP_MAXSEG: i32 = 0x02;
pub const EINTR: i32 = 4;
pub const EAGAIN: i32 = 35;
pub const EWOULDBLOCK: i32 = EAGAIN;
pub const EINPROGRESS: i32 = 36;
pub const ETIMEDOUT: i32 = 60;
pub const ECONNREFUSED: i32 = 61;

/// `struct sockaddr_in` (BSD 4.4 layout, with `sin_len`).
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SockAddrIn {
    pub sin_len: u8,
    pub sin_family: u8,
    /// Network byte order (big-endian, as the CPU is).
    pub sin_port: u16,
    /// Network byte order.
    pub sin_addr: u32,
    pub sin_zero: [u8; 8],
}
const _: () = assert!(core::mem::size_of::<SockAddrIn>() == 16);

impl SockAddrIn {
    /// An IPv4 address and port.
    pub fn v4(ip: [u8; 4], port: u16) -> Self {
        SockAddrIn {
            sin_len: 16,
            sin_family: AF_INET as u8,
            sin_port: port,
            sin_addr: u32::from_be_bytes(ip),
            sin_zero: [0; 8],
        }
    }
    pub fn ip(&self) -> [u8; 4] {
        self.sin_addr.to_be_bytes()
    }
}

// ---- libraries/bsdsocket.h ----

pub const SBTF_VAL: u32 = 0x0000;
pub const SBTF_REF: u32 = 0x8000;
pub const SBTB_CODE: u32 = 1;
pub const SBTS_CODE: u32 = 0x3FFF;
pub const SBTF_GET: u32 = 0;
pub const SBTF_SET: u32 = 1;
/// `SBTM_GETREF(code)`: read the value into `*ti_Data`.
pub const fn sbtm_getref(code: u32) -> Tag {
    TAG_USER | SBTF_REF | ((code & SBTS_CODE) << SBTB_CODE) | SBTF_GET
}
/// `SBTM_SETVAL(code)`: set the value to `ti_Data`.
pub const fn sbtm_setval(code: u32) -> Tag {
    TAG_USER | SBTF_VAL | ((code & SBTS_CODE) << SBTB_CODE) | SBTF_SET
}
pub const SBTC_BREAKMASK: u32 = 1;
pub const SBTC_HAVE_INTERFACE_API: u32 = 47;
pub const SBTC_HAVE_STATUS_API: u32 = 53;

/// `IFQ_State` values.
pub const SM_Down: i32 = 2;
pub const SM_Up: i32 = 3;

#[allow(non_upper_case_globals)]
mod ifq {
    use crate::sys::{Tag, TAG_USER};
    pub const IFQ_BASE: Tag = TAG_USER + 1900;
    pub const IFQ_DeviceName: Tag = IFQ_BASE + 1;
    pub const IFQ_DeviceUnit: Tag = IFQ_BASE + 2;
    pub const IFQ_HardwareAddressSize: Tag = IFQ_BASE + 3;
    pub const IFQ_HardwareAddress: Tag = IFQ_BASE + 4;
    pub const IFQ_MTU: Tag = IFQ_BASE + 5;
    pub const IFQ_BPS: Tag = IFQ_BASE + 6;
    pub const IFQ_HardwareType: Tag = IFQ_BASE + 7;
    pub const IFQ_PacketsReceived: Tag = IFQ_BASE + 8;
    pub const IFQ_PacketsSent: Tag = IFQ_BASE + 9;
    pub const IFQ_BadData: Tag = IFQ_BASE + 10;
    pub const IFQ_Overruns: Tag = IFQ_BASE + 11;
    pub const IFQ_UnknownTypes: Tag = IFQ_BASE + 12;
    pub const IFQ_LastStart: Tag = IFQ_BASE + 13;
    pub const IFQ_Address: Tag = IFQ_BASE + 14;
    pub const IFQ_DestinationAddress: Tag = IFQ_BASE + 15;
    pub const IFQ_BroadcastAddress: Tag = IFQ_BASE + 16;
    pub const IFQ_NetMask: Tag = IFQ_BASE + 17;
    pub const IFQ_Metric: Tag = IFQ_BASE + 18;
    pub const IFQ_State: Tag = IFQ_BASE + 19;
    pub const IFQ_AddressBindType: Tag = IFQ_BASE + 20;
    pub const IFQ_AddressLeaseExpires: Tag = IFQ_BASE + 21;
    pub const IFQ_PrimaryDNSAddress: Tag = IFQ_BASE + 22;
    pub const IFQ_SecondaryDNSAddress: Tag = IFQ_BASE + 23;
    pub const IFQ_NumReadRequests: Tag = IFQ_BASE + 24;
    pub const IFQ_MaxReadRequests: Tag = IFQ_BASE + 25;
    pub const IFQ_NumWriteRequests: Tag = IFQ_BASE + 26;
    pub const IFQ_MaxWriteRequests: Tag = IFQ_BASE + 27;
    pub const IFQ_GetBytesIn: Tag = IFQ_BASE + 28;
    pub const IFQ_GetBytesOut: Tag = IFQ_BASE + 29;
    pub const IFQ_GetDebugMode: Tag = IFQ_BASE + 30;
    pub const IFQ_GetSANA2CopyStats: Tag = IFQ_BASE + 31;
    pub const IFQ_NumReadRequestsPending: Tag = IFQ_BASE + 32;
    pub const IFQ_NumWriteRequestsPending: Tag = IFQ_BASE + 33;
    pub const IFQ_HardwareMTU: Tag = IFQ_BASE + 34;
    pub const IFQ_OutputDrops: Tag = IFQ_BASE + 35;
    pub const IFQ_InputDrops: Tag = IFQ_BASE + 36;
    pub const IFQ_OutputErrors: Tag = IFQ_BASE + 37;
    pub const IFQ_InputErrors: Tag = IFQ_BASE + 38;
    pub const IFQ_OutputMulticasts: Tag = IFQ_BASE + 39;
    pub const IFQ_InputMulticasts: Tag = IFQ_BASE + 40;
    pub const IFQ_IPDrops: Tag = IFQ_BASE + 41;
    pub const IFQ_ARPDrops: Tag = IFQ_BASE + 42;
}
pub use ifq::*;

/// `struct SANA2CopyStats` (`IFQ_GetSANA2CopyStats`): how many times each
/// SANA-II copy function was called, not bytes.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct SANA2CopyStats {
    pub s2cs_DMAIn: u32,
    pub s2cs_DMAOut: u32,
    pub s2cs_ByteIn: u32,
    pub s2cs_ByteOut: u32,
    pub s2cs_WordOut: u32,
}

pub const NETWORKSTATUS_VERSION: i32 = 1;
pub const NETSTATUS_icmp: i32 = 0;
pub const NETSTATUS_igmp: i32 = 1;
pub const NETSTATUS_ip: i32 = 2;
pub const NETSTATUS_mb: i32 = 3;
pub const NETSTATUS_mrt: i32 = 4;
pub const NETSTATUS_rt: i32 = 5;
pub const NETSTATUS_tcp: i32 = 6;
pub const NETSTATUS_udp: i32 = 7;
pub const NETSTATUS_tcp_sockets: i32 = 9;
pub const NETSTATUS_udp_sockets: i32 = 10;

/// `struct tcpstat` (netinet/tcp_var.h), in order.
pub const TCPSTAT_FIELDS: &[&str] = &[
    "tcps_connattempt", "tcps_accepts", "tcps_connects", "tcps_drops", "tcps_conndrops",
    "tcps_closed", "tcps_segstimed", "tcps_rttupdated", "tcps_delack", "tcps_timeoutdrop",
    "tcps_rexmttimeo", "tcps_persisttimeo", "tcps_keeptimeo", "tcps_keepprobe", "tcps_keepdrops",
    "tcps_sndtotal", "tcps_sndpack", "tcps_sndbyte", "tcps_sndrexmitpack", "tcps_sndrexmitbyte",
    "tcps_sndacks", "tcps_sndprobe", "tcps_sndurg", "tcps_sndwinup", "tcps_sndctrl",
    "tcps_rcvtotal", "tcps_rcvpack", "tcps_rcvbyte", "tcps_rcvbadsum", "tcps_rcvbadoff",
    "tcps_rcvshort", "tcps_rcvduppack", "tcps_rcvdupbyte", "tcps_rcvpartduppack",
    "tcps_rcvpartdupbyte", "tcps_rcvoopack", "tcps_rcvoobyte", "tcps_rcvpackafterwin",
    "tcps_rcvbyteafterwin", "tcps_rcvafterclose", "tcps_rcvwinprobe", "tcps_rcvdupack",
    "tcps_rcvacktoomuch", "tcps_rcvackpack", "tcps_rcvackbyte", "tcps_rcvwinupd",
    "tcps_pawsdrop", "tcps_predack", "tcps_preddat", "tcps_pcbcachemiss", "tcps_persistdrop",
    "tcps_badsyn",
];

/// `struct ipstat` (netinet/ip_var.h), in order.
pub const IPSTAT_FIELDS: &[&str] = &[
    "ips_total", "ips_badsum", "ips_tooshort", "ips_toosmall", "ips_badhlen", "ips_badlen",
    "ips_fragments", "ips_fragdropped", "ips_fragtimeout", "ips_forward", "ips_cantforward",
    "ips_redirectsent", "ips_noproto", "ips_delivered", "ips_localout", "ips_odropped",
    "ips_reassembled", "ips_fragmented", "ips_ofragments", "ips_cantfrag", "ips_badoptions",
    "ips_noroute", "ips_badvers", "ips_rawout",
];

/// `struct udpstat` (netinet/udp_var.h), in order.
pub const UDPSTAT_FIELDS: &[&str] = &[
    "udps_ipackets", "udps_hdrops", "udps_badsum", "udps_badlen", "udps_noport",
    "udps_noportbcast", "udps_fullsock", "udpps_pcbcachemiss", "udps_opackets",
];

/// `struct mbstat` (sys/mbuf.h), in order.
pub const MBSTAT_FIELDS: &[&str] = &[
    "m_mbufs", "m_clusters", "m_spare", "m_clfree", "m_drops", "m_wait", "m_drain",
];

/// The index of `name` in one of the `*_FIELDS` tables.
pub fn field_index(table: &[&str], name: &str) -> Option<usize> {
    table.iter().position(|f| *f == name)
}

/// `struct protocol_connection_data` (pack 2): one entry per socket from
/// `NETSTATUS_tcp_sockets` / `NETSTATUS_udp_sockets`.
#[repr(C, packed(2))]
#[derive(Clone, Copy, Default)]
pub struct ProtocolConnectionData {
    pub pcd_foreign_address: u32,
    pub pcd_foreign_port: u16,
    pub pcd_local_address: u32,
    pub pcd_local_port: u16,
    pub pcd_receive_queue_size: u32,
    pub pcd_send_queue_size: u32,
    pub pcd_tcp_state: i32,
}
const _: () = assert!(core::mem::size_of::<ProtocolConnectionData>() == 24);

// ---- The library ----

/// An opened bsdsocket.library: its base and `main` interface.
pub struct SocketLib {
    _t: crate::track::Tracked,
    base: *mut Library,
    iface: *mut SocketIFace,
}

fn cstr(s: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(s.len() + 1);
    v.extend_from_slice(s.as_bytes());
    v.push(0);
    v
}

impl SocketLib {
    /// Open `bsdsocket.library` (at least `version`) and its interface.
    #[track_caller]
    pub fn open(version: u32) -> Result<Self> {
        Self::open_named("bsdsocket.library", version)
    }

    /// Open another library with bsdsocket.library's interface (a stack
    /// under its own name, or a path).
    #[track_caller]
    pub fn open_named(name: &str, version: u32) -> Result<Self> {
        let n = crate::cstr::c_copy(name.as_bytes())?;
        // SAFETY: plain exec calls with NUL-terminated names.
        unsafe {
            let base = sys::exec_open_library(n.as_ptr(), version);
            if base.is_null() {
                return Err(AmigaError::Device("the socket library did not open"));
            }
            let iface = sys::exec_get_interface(base, b"main\0".as_ptr(), 1, null_mut());
            if iface.is_null() {
                sys::exec_close_library(base);
                return Err(AmigaError::Device("bsdsocket.library has no main interface"));
            }
            Ok(SocketLib { base, iface: iface as *mut SocketIFace, _t: crate::track::register("bsdsocket base", base as usize, iface as usize, release_socket_lib) })
        }
    }

    /// The raw interface, for calls this type does not wrap.
    pub fn iface(&self) -> *mut SocketIFace {
        self.iface
    }

    /// The library's version and revision (`lib_Version`, `lib_Revision`).
    pub fn version(&self) -> (u16, u16) {
        // SAFETY: struct Library: a 14-byte Node, two UBYTEs, two UWORDs
        // (NegSize, PosSize), then lib_Version and lib_Revision (pack 2).
        unsafe {
            let p = self.base as *const u8;
            ((p.add(20) as *const u16).read_unaligned(), (p.add(22) as *const u16).read_unaligned())
        }
    }

    /// `getsockname` for an IPv4 socket.
    pub fn local_addr(&self, fd: i32) -> Result<SockAddrIn> {
        let mut a = SockAddrIn::default();
        let mut len: u32 = 16;
        // SAFETY: `a` has room for a sockaddr_in.
        let r = unsafe { ((*self.iface).getsockname)(self.iface, fd, &mut a as *mut _ as *mut c_void, &mut len) };
        self.check(r).map(|_| a)
    }

    /// An interface's IPv4 address and netmask (`IFQ_Address`, `IFQ_NetMask`).
    pub fn interface_ipv4(&self, name: &str) -> Result<([u8; 4], [u8; 4])> {
        let mut a = SockAddrIn::default();
        let mut m = SockAddrIn::default();
        // SAFETY: both tags take a struct sockaddr_in *.
        unsafe {
            self.query_interface(name, &[
                TagItem { ti_Tag: IFQ_Address, ti_Data: &mut a as *mut _ as usize as u32 },
                TagItem { ti_Tag: IFQ_NetMask, ti_Data: &mut m as *mut _ as usize as u32 },
            ])?;
        }
        Ok((a.ip(), m.ip()))
    }

    /// This base's `errno` after a failed call.
    pub fn errno(&self) -> i32 {
        // SAFETY: a valid interface.
        unsafe { ((*self.iface).Errno)(self.iface) }
    }

    fn check(&self, r: i32) -> Result<i32> {
        if r < 0 { Err(AmigaError::IoError(self.errno())) } else { Ok(r) }
    }

    /// `SocketBaseTagList` with one `SBTM_GETREF(code)` tag: the value, or
    /// an error if the library does not know the code.
    pub fn get_base_value(&self, code: u32) -> Result<u32> {
        let mut v: u32 = 0;
        let tags = [
            TagItem { ti_Tag: sbtm_getref(code), ti_Data: &mut v as *mut u32 as usize as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: a valid interface and a terminated tag list.
        let r = unsafe { ((*self.iface).SocketBaseTagList)(self.iface, tags.as_ptr() as *mut TagItem) };
        if r != 0 { Err(AmigaError::IoError(r)) } else { Ok(v) }
    }

    /// `SocketBaseTagList` with one `SBTM_SETVAL(code)` tag.
    pub fn set_base_value(&self, code: u32, value: u32) -> Result<()> {
        let tags = [
            TagItem { ti_Tag: sbtm_setval(code), ti_Data: value },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: as above.
        let r = unsafe { ((*self.iface).SocketBaseTagList)(self.iface, tags.as_ptr() as *mut TagItem) };
        if r != 0 { Err(AmigaError::IoError(r)) } else { Ok(()) }
    }

    // ---- Sockets ----

    pub fn socket(&self, domain: i32, kind: i32, protocol: i32) -> Result<i32> {
        // SAFETY: a valid interface.
        self.check(unsafe { ((*self.iface).socket)(self.iface, domain, kind, protocol) })
    }
    pub fn connect(&self, fd: i32, addr: &SockAddrIn) -> Result<()> {
        // SAFETY: `addr` is a 16-byte sockaddr_in.
        let r = unsafe { ((*self.iface).connect)(self.iface, fd, addr as *const _ as *mut c_void, 16) };
        self.check(r).map(|_| ())
    }
    pub fn bind(&self, fd: i32, addr: &SockAddrIn) -> Result<()> {
        // SAFETY: as above.
        let r = unsafe { ((*self.iface).bind)(self.iface, fd, addr as *const _ as *mut c_void, 16) };
        self.check(r).map(|_| ())
    }
    pub fn listen(&self, fd: i32, backlog: i32) -> Result<()> {
        // SAFETY: a valid interface.
        self.check(unsafe { ((*self.iface).listen)(self.iface, fd, backlog) }).map(|_| ())
    }
    pub fn accept(&self, fd: i32) -> Result<(i32, SockAddrIn)> {
        let mut a = SockAddrIn::default();
        let mut len: u32 = 16;
        // SAFETY: `a` has room for a sockaddr_in.
        let r = unsafe { ((*self.iface).accept)(self.iface, fd, &mut a as *mut _ as *mut c_void, &mut len) };
        self.check(r).map(|s| (s, a))
    }
    /// `send`: the bytes accepted.
    pub fn send(&self, fd: i32, buf: &[u8], flags: i32) -> Result<usize> {
        // SAFETY: the buffer is valid for its length.
        let r = unsafe { ((*self.iface).send)(self.iface, fd, buf.as_ptr() as *mut c_void, buf.len() as i32, flags) };
        self.check(r).map(|n| n as usize)
    }
    /// `recv`: the bytes received, 0 at the end of the stream.
    pub fn recv(&self, fd: i32, buf: &mut [u8], flags: i32) -> Result<usize> {
        // SAFETY: as above.
        let r = unsafe { ((*self.iface).recv)(self.iface, fd, buf.as_mut_ptr() as *mut c_void, buf.len() as i32, flags) };
        self.check(r).map(|n| n as usize)
    }
    pub fn shutdown(&self, fd: i32, how: i32) -> Result<()> {
        // SAFETY: a valid interface.
        self.check(unsafe { ((*self.iface).shutdown)(self.iface, fd, how) }).map(|_| ())
    }
    /// setsockopt with a 4-byte value. SO_RCVTIMEO, SO_SNDTIMEO and
    /// SO_LINGER take a timeval or a struct linger, not an int: they are
    /// refused here (`net::Socket` has typed setters for them).
    pub fn set_opt(&self, fd: i32, level: i32, name: i32, value: i32) -> Result<()> {
        if level == SOL_SOCKET && matches!(name, SO_RCVTIMEO | SO_SNDTIMEO | 0x0080) {
            return Err(AmigaError::InvalidArgument("this option is not a 4-byte value"));
        }
        let v = value;
        // SAFETY: a 4-byte option value.
        let r = unsafe { ((*self.iface).setsockopt)(self.iface, fd, level, name, &v as *const i32 as *mut c_void, 4) };
        self.check(r).map(|_| ())
    }
    /// getsockopt of a 4-byte value (the timeouts and linger are refused,
    /// as in `set_opt`).
    pub fn get_opt(&self, fd: i32, level: i32, name: i32) -> Result<i32> {
        if level == SOL_SOCKET && matches!(name, SO_RCVTIMEO | SO_SNDTIMEO | 0x0080) {
            return Err(AmigaError::InvalidArgument("this option is not a 4-byte value"));
        }
        let mut v: i32 = 0;
        let mut len: u32 = 4;
        // SAFETY: as above.
        let r = unsafe { ((*self.iface).getsockopt)(self.iface, fd, level, name, &mut v as *mut i32 as *mut c_void, &mut len) };
        self.check(r).map(|_| v)
    }
    pub fn close(&self, fd: i32) -> Result<()> {
        // SAFETY: a valid interface.
        self.check(unsafe { ((*self.iface).CloseSocket)(self.iface, fd) }).map(|_| ())
    }

    // ---- Interfaces and statistics ----

    /// The names of the stack's interfaces (`ObtainInterfaceList`).
    pub fn interfaces(&self) -> Result<Vec<String>> {
        // SAFETY: the list is the library's copy, read and released here.
        unsafe {
            let list = ((*self.iface).ObtainInterfaceList)(self.iface);
            if list.is_null() {
                return Err(AmigaError::AllocationFailed);
            }
            let mut names = Vec::new();
            // struct List starts with lh_Head.
            let mut n = *(list as *const *mut Node);
            while !n.is_null() {
                let succ = core::ptr::addr_of!((*n).ln_Succ).read_unaligned();
                if succ.is_null() {
                    break;
                }
                let name = core::ptr::addr_of!((*n).ln_Name).read_unaligned();
                if !name.is_null() {
                    let mut len = 0;
                    while *name.add(len) != 0 {
                        len += 1;
                    }
                    let bytes = core::slice::from_raw_parts(name, len);
                    names.push(String::from_utf8_lossy(bytes).into_owned());
                }
                n = succ;
            }
            ((*self.iface).ReleaseInterfaceList)(self.iface, list);
            Ok(names)
        }
    }

    /// `QueryInterfaceTagList(name, tags)`: each tag's `ti_Data` points to
    /// where its answer goes.
    ///
    /// # Safety
    /// Every `ti_Data` must point to storage of the type its tag documents.
    pub unsafe fn query_interface(&self, name: &str, tags: &[TagItem]) -> Result<()> {
        let n = cstr(name);
        let mut t: Vec<TagItem> = tags.iter().map(|x| TagItem { ti_Tag: x.ti_Tag, ti_Data: x.ti_Data }).collect();
        t.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        // SAFETY: an open interface, a NUL-terminated name, a TAG_DONE-ended
        // list whose ti_Data point where the caller's contract says.
        let r = unsafe { ((*self.iface).QueryInterfaceTagList)(self.iface, n.as_ptr() as *mut u8, t.as_mut_ptr()) };
        if r != 0 { Err(AmigaError::IoError(self.errno())) } else { Ok(()) }
    }

    /// One 32-bit answer (`LONG *` / `ULONG *` tags).
    pub fn query_u32(&self, name: &str, tag: Tag) -> Result<u32> {
        let mut v: u32 = 0;
        // SAFETY: the caller's tag takes a 32-bit destination.
        unsafe { self.query_interface(name, &[TagItem { ti_Tag: tag, ti_Data: &mut v as *mut u32 as usize as u32 }])? };
        Ok(v)
    }

    /// One `SBQUAD_T` answer (`IFQ_GetBytesIn`, `IFQ_GetBytesOut`).
    pub fn query_u64(&self, name: &str, tag: Tag) -> Result<u64> {
        let mut v: [u32; 2] = [0; 2];
        // SAFETY: SBQUAD_T is { sbq_High, sbq_Low }.
        unsafe { self.query_interface(name, &[TagItem { ti_Tag: tag, ti_Data: v.as_mut_ptr() as usize as u32 }])? };
        Ok(((v[0] as u64) << 32) | v[1] as u64)
    }

    /// The SANA-II device name and unit behind an interface.
    pub fn interface_device(&self, name: &str) -> Result<(String, u32)> {
        let mut p: *const u8 = core::ptr::null();
        let mut unit: u32 = 0;
        // SAFETY: IFQ_DeviceName takes STRPTR *, IFQ_DeviceUnit LONG *.
        unsafe {
            self.query_interface(name, &[
                TagItem { ti_Tag: IFQ_DeviceName, ti_Data: &mut p as *mut *const u8 as usize as u32 },
                TagItem { ti_Tag: IFQ_DeviceUnit, ti_Data: &mut unit as *mut u32 as usize as u32 },
            ])?;
            if p.is_null() {
                return Err(AmigaError::NullPointer);
            }
            let mut len = 0;
            while *p.add(len) != 0 {
                len += 1;
            }
            Ok((String::from_utf8_lossy(core::slice::from_raw_parts(p, len)).into_owned(), unit))
        }
    }

    /// `IFQ_GetSANA2CopyStats`.
    pub fn copy_stats(&self, name: &str) -> Result<SANA2CopyStats> {
        let mut s = SANA2CopyStats::default();
        // SAFETY: the tag takes a struct SANA2CopyStats *.
        unsafe { self.query_interface(name, &[TagItem { ti_Tag: IFQ_GetSANA2CopyStats, ti_Data: &mut s as *mut _ as usize as u32 }])? };
        Ok(s)
    }

    /// `GetNetworkStatistics(kind, NETWORKSTATUS_VERSION, out, size)`
    /// into counters: the number of bytes copied.
    pub fn network_stats(&self, kind: i32, out: &mut [u32]) -> Result<usize> {
        // SAFETY: `out` is writable for its length in bytes.
        let r = unsafe {
            ((*self.iface).GetNetworkStatistics)(self.iface, kind, NETWORKSTATUS_VERSION,
                out.as_mut_ptr() as *mut c_void, (out.len() * 4) as i32)
        };
        self.check(r).map(|n| n as usize)
    }

    /// Every TCP (`NETSTATUS_tcp_sockets`) or UDP socket's addresses and
    /// queue sizes.
    pub fn connections(&self, kind: i32) -> Result<Vec<ProtocolConnectionData>> {
        // SAFETY: a NULL destination asks for the size; then a buffer of it.
        unsafe {
            let need = ((*self.iface).GetNetworkStatistics)(self.iface, kind, NETWORKSTATUS_VERSION, null_mut(), 0);
            let need = self.check(need)? as usize;
            let n = need / core::mem::size_of::<ProtocolConnectionData>();
            let mut v: Vec<ProtocolConnectionData> = alloc::vec![ProtocolConnectionData::default(); n];
            if n == 0 {
                return Ok(v);
            }
            let got = ((*self.iface).GetNetworkStatistics)(self.iface, kind, NETWORKSTATUS_VERSION,
                v.as_mut_ptr() as *mut c_void, (n * core::mem::size_of::<ProtocolConnectionData>()) as i32);
            let got = self.check(got)? as usize;
            v.truncate(got / core::mem::size_of::<ProtocolConnectionData>());
            Ok(v)
        }
    }
}

/// Close a registered socket library base, with every socket of it (the
/// panic path; see `crate::track`).
unsafe fn release_socket_lib(base: usize, iface: usize) {
    // SAFETY: the registry calls this once, for a base still open.
    unsafe {
        sys::exec_drop_interface(iface as *mut Interface);
        sys::exec_close_library(base as *mut Library);
    }
}

impl Drop for SocketLib {
    fn drop(&mut self) {
        // SAFETY: opened in `open`; sockets of this base close with it.
        unsafe {
            sys::exec_drop_interface(self.iface as *mut Interface);
            sys::exec_close_library(self.base);
        }
    }
}

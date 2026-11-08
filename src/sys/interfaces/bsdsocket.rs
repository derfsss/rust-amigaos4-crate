//! SocketIFace interface vtable struct (bsdsocket.library, Roadshow).
//!
//! Generated from the SDK's netinclude/interfaces/bsdsocket.h; slot order is
//! checked by tests/sdk_audit.rs. Network structures (sockaddr, hostent,
//! mbuf and the rest) are passed as untyped pointers.

use core::ffi::c_void;

use crate::sys::types::*;

// ---- SocketIFace (bsdsocket.h) ----

#[repr(C)]
pub struct SocketIFace {
    pub data: InterfaceData,

    pub Obtain: unsafe extern "C" fn(*mut SocketIFace) -> u32, // 0: 60
    pub Release: unsafe extern "C" fn(*mut SocketIFace) -> u32, // 1: 64
    pub Expunge: unsafe extern "C" fn(*mut SocketIFace), // 2: 68
    pub Clone: unsafe extern "C" fn(*mut SocketIFace) -> *mut Interface, // 3: 72
    pub socket: unsafe extern "C" fn(*mut SocketIFace, i32, i32, i32) -> i32, // 4: 76
    pub bind: unsafe extern "C" fn(*mut SocketIFace, i32, *mut c_void, u32) -> i32, // 5: 80
    pub listen: unsafe extern "C" fn(*mut SocketIFace, i32, i32) -> i32, // 6: 84
    pub accept: unsafe extern "C" fn(*mut SocketIFace, i32, *mut c_void, *mut u32) -> i32, // 7: 88
    pub connect: unsafe extern "C" fn(*mut SocketIFace, i32, *mut c_void, u32) -> i32, // 8: 92
    pub sendto: unsafe extern "C" fn(*mut SocketIFace, i32, APTR, i32, i32, *mut c_void, u32) -> i32, // 9: 96
    pub send: unsafe extern "C" fn(*mut SocketIFace, i32, APTR, i32, i32) -> i32, // 10: 100
    pub recvfrom: unsafe extern "C" fn(*mut SocketIFace, i32, APTR, i32, i32, *mut c_void, *mut u32) -> i32, // 11: 104
    pub recv: unsafe extern "C" fn(*mut SocketIFace, i32, APTR, i32, i32) -> i32, // 12: 108
    pub shutdown: unsafe extern "C" fn(*mut SocketIFace, i32, i32) -> i32, // 13: 112
    pub setsockopt: unsafe extern "C" fn(*mut SocketIFace, i32, i32, i32, APTR, u32) -> i32, // 14: 116
    pub getsockopt: unsafe extern "C" fn(*mut SocketIFace, i32, i32, i32, APTR, *mut u32) -> i32, // 15: 120
    pub getsockname: unsafe extern "C" fn(*mut SocketIFace, i32, *mut c_void, *mut u32) -> i32, // 16: 124
    pub getpeername: unsafe extern "C" fn(*mut SocketIFace, i32, *mut c_void, *mut u32) -> i32, // 17: 128
    pub IoctlSocket: unsafe extern "C" fn(*mut SocketIFace, i32, u32, APTR) -> i32, // 18: 132
    pub CloseSocket: unsafe extern "C" fn(*mut SocketIFace, i32) -> i32, // 19: 136
    pub WaitSelect: unsafe extern "C" fn(*mut SocketIFace, i32, APTR, APTR, APTR, *mut c_void, *mut u32) -> i32, // 20: 140
    pub SetSocketSignals: unsafe extern "C" fn(*mut SocketIFace, u32, u32, u32), // 21: 144
    pub getdtablesize: unsafe extern "C" fn(*mut SocketIFace) -> i32, // 22: 148
    pub ObtainSocket: unsafe extern "C" fn(*mut SocketIFace, i32, i32, i32, i32) -> i32, // 23: 152
    pub ReleaseSocket: unsafe extern "C" fn(*mut SocketIFace, i32, i32) -> i32, // 24: 156
    pub ReleaseCopyOfSocket: unsafe extern "C" fn(*mut SocketIFace, i32, i32) -> i32, // 25: 160
    pub Errno: unsafe extern "C" fn(*mut SocketIFace) -> i32, // 26: 164
    pub SetErrnoPtr: unsafe extern "C" fn(*mut SocketIFace, APTR, i32), // 27: 168
    pub Inet_NtoA: unsafe extern "C" fn(*mut SocketIFace, u32) -> STRPTR, // 28: 172
    pub inet_addr: unsafe extern "C" fn(*mut SocketIFace, STRPTR) -> u32, // 29: 176
    pub Inet_LnaOf: unsafe extern "C" fn(*mut SocketIFace, u32) -> u32, // 30: 180
    pub Inet_NetOf: unsafe extern "C" fn(*mut SocketIFace, u32) -> u32, // 31: 184
    pub Inet_MakeAddr: unsafe extern "C" fn(*mut SocketIFace, u32, u32) -> u32, // 32: 188
    pub inet_network: unsafe extern "C" fn(*mut SocketIFace, STRPTR) -> u32, // 33: 192
    pub gethostbyname: unsafe extern "C" fn(*mut SocketIFace, STRPTR) -> *mut c_void, // 34: 196
    pub gethostbyaddr: unsafe extern "C" fn(*mut SocketIFace, STRPTR, i32, i32) -> *mut c_void, // 35: 200
    pub getnetbyname: unsafe extern "C" fn(*mut SocketIFace, STRPTR) -> *mut c_void, // 36: 204
    pub getnetbyaddr: unsafe extern "C" fn(*mut SocketIFace, u32, i32) -> *mut c_void, // 37: 208
    pub getservbyname: unsafe extern "C" fn(*mut SocketIFace, STRPTR, STRPTR) -> *mut c_void, // 38: 212
    pub getservbyport: unsafe extern "C" fn(*mut SocketIFace, i32, STRPTR) -> *mut c_void, // 39: 216
    pub getprotobyname: unsafe extern "C" fn(*mut SocketIFace, STRPTR) -> *mut c_void, // 40: 220
    pub getprotobynumber: unsafe extern "C" fn(*mut SocketIFace, i32) -> *mut c_void, // 41: 224
    pub vsyslog: unsafe extern "C" fn(*mut SocketIFace, i32, STRPTR, APTR), // 42: 228
    pub syslog: APTR, // 43: 232 (varargs)
    pub Dup2Socket: unsafe extern "C" fn(*mut SocketIFace, i32, i32) -> i32, // 44: 236
    pub sendmsg: unsafe extern "C" fn(*mut SocketIFace, i32, *mut c_void, i32) -> i32, // 45: 240
    pub recvmsg: unsafe extern "C" fn(*mut SocketIFace, i32, *mut c_void, i32) -> i32, // 46: 244
    pub gethostname: unsafe extern "C" fn(*mut SocketIFace, STRPTR, i32) -> i32, // 47: 248
    pub gethostid: unsafe extern "C" fn(*mut SocketIFace) -> u32, // 48: 252
    pub SocketBaseTagList: unsafe extern "C" fn(*mut SocketIFace, *mut TagItem) -> i32, // 49: 256
    pub SocketBaseTags: APTR, // 50: 260 (varargs)
    pub GetSocketEvents: unsafe extern "C" fn(*mut SocketIFace, *mut u32) -> i32, // 51: 264
    pub Reserved1: APTR, // 52: 268
    pub Reserved2: APTR, // 53: 272
    pub Reserved3: APTR, // 54: 276
    pub Reserved4: APTR, // 55: 280
    pub Reserved5: APTR, // 56: 284
    pub Reserved6: APTR, // 57: 288
    pub Reserved7: APTR, // 58: 292
    pub Reserved8: APTR, // 59: 296
    pub Reserved9: APTR, // 60: 300
    pub Reserved10: APTR, // 61: 304
    pub bpf_open: unsafe extern "C" fn(*mut SocketIFace, i32) -> i32, // 62: 308
    pub bpf_close: unsafe extern "C" fn(*mut SocketIFace, i32) -> i32, // 63: 312
    pub bpf_read: unsafe extern "C" fn(*mut SocketIFace, i32, APTR, i32) -> i32, // 64: 316
    pub bpf_write: unsafe extern "C" fn(*mut SocketIFace, i32, APTR, i32) -> i32, // 65: 320
    pub bpf_set_notify_mask: unsafe extern "C" fn(*mut SocketIFace, i32, u32) -> i32, // 66: 324
    pub bpf_set_interrupt_mask: unsafe extern "C" fn(*mut SocketIFace, i32, u32) -> i32, // 67: 328
    pub bpf_ioctl: unsafe extern "C" fn(*mut SocketIFace, i32, u32, APTR) -> i32, // 68: 332
    pub bpf_data_waiting: unsafe extern "C" fn(*mut SocketIFace, i32) -> i32, // 69: 336
    pub AddRouteTagList: unsafe extern "C" fn(*mut SocketIFace, *mut TagItem) -> i32, // 70: 340
    pub AddRouteTags: APTR, // 71: 344 (varargs)
    pub DeleteRouteTagList: unsafe extern "C" fn(*mut SocketIFace, *mut TagItem) -> i32, // 72: 348
    pub DeleteRouteTags: APTR, // 73: 352 (varargs)
    pub ChangeRouteTagList: unsafe extern "C" fn(*mut SocketIFace, *mut TagItem) -> i32, // 74: 356
    pub ChangeRouteTags: APTR, // 75: 360 (varargs)
    pub FreeRouteInfo: unsafe extern "C" fn(*mut SocketIFace, *mut c_void), // 76: 364
    pub GetRouteInfo: unsafe extern "C" fn(*mut SocketIFace, i32, i32) -> *mut c_void, // 77: 368
    pub AddInterfaceTagList: unsafe extern "C" fn(*mut SocketIFace, STRPTR, STRPTR, i32, *mut TagItem) -> i32, // 78: 372
    pub AddInterfaceTags: APTR, // 79: 376 (varargs)
    pub ConfigureInterfaceTagList: unsafe extern "C" fn(*mut SocketIFace, STRPTR, *mut TagItem) -> i32, // 80: 380
    pub ConfigureInterfaceTags: APTR, // 81: 384 (varargs)
    pub ReleaseInterfaceList: unsafe extern "C" fn(*mut SocketIFace, *mut List), // 82: 388
    pub ObtainInterfaceList: unsafe extern "C" fn(*mut SocketIFace) -> *mut List, // 83: 392
    pub QueryInterfaceTagList: unsafe extern "C" fn(*mut SocketIFace, STRPTR, *mut TagItem) -> i32, // 84: 396
    pub QueryInterfaceTags: APTR, // 85: 400 (varargs)
    pub CreateAddrAllocMessageA: unsafe extern "C" fn(*mut SocketIFace, i32, i32, STRPTR, *mut *mut c_void, *mut TagItem) -> i32, // 86: 404
    pub CreateAddrAllocMessage: APTR, // 87: 408 (varargs)
    pub DeleteAddrAllocMessage: unsafe extern "C" fn(*mut SocketIFace, *mut c_void), // 88: 412
    pub BeginInterfaceConfig: unsafe extern "C" fn(*mut SocketIFace, *mut c_void), // 89: 416
    pub AbortInterfaceConfig: unsafe extern "C" fn(*mut SocketIFace, *mut c_void), // 90: 420
    pub AddNetMonitorHookTagList: unsafe extern "C" fn(*mut SocketIFace, i32, *mut Hook, *mut TagItem) -> i32, // 91: 424
    pub AddNetMonitorHookTags: APTR, // 92: 428 (varargs)
    pub RemoveNetMonitorHook: unsafe extern "C" fn(*mut SocketIFace, *mut Hook), // 93: 432
    pub GetNetworkStatistics: unsafe extern "C" fn(*mut SocketIFace, i32, i32, APTR, i32) -> i32, // 94: 436
    pub AddDomainNameServer: unsafe extern "C" fn(*mut SocketIFace, STRPTR) -> i32, // 95: 440
    pub RemoveDomainNameServer: unsafe extern "C" fn(*mut SocketIFace, STRPTR) -> i32, // 96: 444
    pub ReleaseDomainNameServerList: unsafe extern "C" fn(*mut SocketIFace, *mut List), // 97: 448
    pub ObtainDomainNameServerList: unsafe extern "C" fn(*mut SocketIFace) -> *mut List, // 98: 452
    pub setnetent: unsafe extern "C" fn(*mut SocketIFace, i32), // 99: 456
    pub endnetent: unsafe extern "C" fn(*mut SocketIFace), // 100: 460
    pub getnetent: unsafe extern "C" fn(*mut SocketIFace) -> *mut c_void, // 101: 464
    pub setprotoent: unsafe extern "C" fn(*mut SocketIFace, i32), // 102: 468
    pub endprotoent: unsafe extern "C" fn(*mut SocketIFace), // 103: 472
    pub getprotoent: unsafe extern "C" fn(*mut SocketIFace) -> *mut c_void, // 104: 476
    pub setservent: unsafe extern "C" fn(*mut SocketIFace, i32), // 105: 480
    pub endservent: unsafe extern "C" fn(*mut SocketIFace), // 106: 484
    pub getservent: unsafe extern "C" fn(*mut SocketIFace) -> *mut c_void, // 107: 488
    pub inet_aton: unsafe extern "C" fn(*mut SocketIFace, STRPTR, *mut c_void) -> i32, // 108: 492
    pub inet_ntop: unsafe extern "C" fn(*mut SocketIFace, i32, APTR, STRPTR, i32) -> STRPTR, // 109: 496
    pub inet_pton: unsafe extern "C" fn(*mut SocketIFace, i32, STRPTR, APTR) -> i32, // 110: 500
    pub In_LocalAddr: unsafe extern "C" fn(*mut SocketIFace, u32) -> i32, // 111: 504
    pub In_CanForward: unsafe extern "C" fn(*mut SocketIFace, u32) -> i32, // 112: 508
    pub mbuf_copym: unsafe extern "C" fn(*mut SocketIFace, *mut c_void, i32, i32) -> *mut c_void, // 113: 512
    pub mbuf_copyback: unsafe extern "C" fn(*mut SocketIFace, *mut c_void, i32, i32, APTR) -> i32, // 114: 516
    pub mbuf_copydata: unsafe extern "C" fn(*mut SocketIFace, *mut c_void, i32, i32, APTR) -> i32, // 115: 520
    pub mbuf_free: unsafe extern "C" fn(*mut SocketIFace, *mut c_void) -> *mut c_void, // 116: 524
    pub mbuf_freem: unsafe extern "C" fn(*mut SocketIFace, *mut c_void), // 117: 528
    pub mbuf_get: unsafe extern "C" fn(*mut SocketIFace) -> *mut c_void, // 118: 532
    pub mbuf_gethdr: unsafe extern "C" fn(*mut SocketIFace) -> *mut c_void, // 119: 536
    pub mbuf_prepend: unsafe extern "C" fn(*mut SocketIFace, *mut c_void, i32) -> *mut c_void, // 120: 540
    pub mbuf_cat: unsafe extern "C" fn(*mut SocketIFace, *mut c_void, *mut c_void) -> i32, // 121: 544
    pub mbuf_adj: unsafe extern "C" fn(*mut SocketIFace, *mut c_void, i32) -> i32, // 122: 548
    pub mbuf_pullup: unsafe extern "C" fn(*mut SocketIFace, *mut c_void, i32) -> *mut c_void, // 123: 552
    pub ProcessIsServer: unsafe extern "C" fn(*mut SocketIFace, *mut Process) -> BOOL, // 124: 556
    pub ObtainServerSocket: unsafe extern "C" fn(*mut SocketIFace) -> i32, // 125: 560
    pub GetDefaultDomainName: unsafe extern "C" fn(*mut SocketIFace, STRPTR, i32) -> BOOL, // 126: 564
    pub SetDefaultDomainName: unsafe extern "C" fn(*mut SocketIFace, STRPTR), // 127: 568
    pub ObtainRoadshowData: unsafe extern "C" fn(*mut SocketIFace, i32) -> *mut List, // 128: 572
    pub ReleaseRoadshowData: unsafe extern "C" fn(*mut SocketIFace, *mut List), // 129: 576
    pub ChangeRoadshowData: unsafe extern "C" fn(*mut SocketIFace, *mut List, STRPTR, u32, APTR) -> BOOL, // 130: 580
    pub RemoveInterface: unsafe extern "C" fn(*mut SocketIFace, STRPTR, i32) -> i32, // 131: 584
    pub gethostbyname_r: unsafe extern "C" fn(*mut SocketIFace, STRPTR, *mut c_void, APTR, u32, *mut i32) -> *mut c_void, // 132: 588
    pub gethostbyaddr_r: unsafe extern "C" fn(*mut SocketIFace, STRPTR, i32, i32, *mut c_void, APTR, u32, *mut i32) -> *mut c_void, // 133: 592
    pub Reserved11: APTR, // 134: 596
    pub Reserved12: APTR, // 135: 600
    pub ipf_open: unsafe extern "C" fn(*mut SocketIFace, i32) -> i32, // 136: 604
    pub ipf_close: unsafe extern "C" fn(*mut SocketIFace, i32) -> i32, // 137: 608
    pub ipf_ioctl: unsafe extern "C" fn(*mut SocketIFace, i32, u32, APTR) -> i32, // 138: 612
    pub ipf_log_read: unsafe extern "C" fn(*mut SocketIFace, i32, APTR, i32) -> i32, // 139: 616
    pub ipf_log_data_waiting: unsafe extern "C" fn(*mut SocketIFace, i32) -> i32, // 140: 620
    pub ipf_set_notify_mask: unsafe extern "C" fn(*mut SocketIFace, i32, u32) -> i32, // 141: 624
    pub ipf_set_interrupt_mask: unsafe extern "C" fn(*mut SocketIFace, i32, u32) -> i32, // 142: 628
    pub freeaddrinfo: unsafe extern "C" fn(*mut SocketIFace, *mut c_void), // 143: 632
    pub getaddrinfo: unsafe extern "C" fn(*mut SocketIFace, STRPTR, STRPTR, *mut c_void, *mut *mut c_void) -> i32, // 144: 636
    pub gai_strerror: unsafe extern "C" fn(*mut SocketIFace, i32) -> STRPTR, // 145: 640
    pub getnameinfo: unsafe extern "C" fn(*mut SocketIFace, *mut c_void, u32, STRPTR, u32, STRPTR, u32, u32) -> i32, // 146: 644
}

//! amigaos4::bsdsocket's constants and statistics field tables against the
//! SDK's netinclude headers (Roadshow). Skips, saying so, without an SDK at
//! `sdk/` or `AMIGA_SDK`.

#![cfg(feature = "bsdsocket")]

use std::fs;
use std::path::PathBuf;

mod common;
use common::{definition, eval, include_dir};

use amigaos4::bsdsocket::*;

fn netinclude() -> Option<PathBuf> {
    let p = include_dir()?.parent()?.join("netinclude");
    if p.is_dir() { Some(p) } else { None }
}

fn read(dir: &PathBuf, p: &str) -> String {
    fs::read_to_string(dir.join(p)).unwrap_or_else(|e| panic!("{}: {}", p, e))
}

/// The member names of `struct <name> { ... };`, in order.
fn struct_fields(text: &str, name: &str) -> Vec<String> {
    let start = text
        .find(&format!("struct\t{} {{", name))
        .or_else(|| text.find(&format!("struct {} {{", name)))
        .or_else(|| text.find(&format!("struct {}\n{{", name)))
        .unwrap_or_else(|| panic!("struct {} not found", name));
    let body = &text[start..];
    let body = &body[body.find('{').unwrap() + 1..body.find("};").unwrap()];
    let mut out = Vec::new();
    for line in body.lines() {
        let l = line.split("/*").next().unwrap_or("").trim();
        if let Some(decl) = l.strip_suffix(';') {
            if let Some(n) = decl.split_whitespace().last() {
                out.push(n.trim_start_matches('*').to_string());
            }
        }
    }
    out
}

#[test]
fn bsdsocket_constants_match_the_sdk() {
    let (Some(inc), Some(net)) = (include_dir(), netinclude()) else {
        eprintln!("[skip] AmigaOS 4 SDK (with netinclude) not found at sdk/ (set AMIGA_SDK to override).");
        return;
    };
    let tagitem = fs::read_to_string(inc.join("utility/tagitem.h")).unwrap();
    let headers = [
        read(&net, "sys/socket.h"),
        read(&net, "netinet/in.h"),
        read(&net, "netinet/tcp.h"),
        read(&net, "sys/errno.h"),
        read(&net, "libraries/bsdsocket.h"),
        tagitem,
    ];
    let texts: Vec<&str> = headers.iter().map(|s| s.as_str()).collect();

    let checks: Vec<(&str, i64)> = vec![
        ("AF_INET", AF_INET as i64),
        ("SOCK_STREAM", SOCK_STREAM as i64),
        ("SOCK_DGRAM", SOCK_DGRAM as i64),
        ("SOL_SOCKET", SOL_SOCKET as i64),
        ("SO_REUSEADDR", SO_REUSEADDR as i64),
        ("SO_SNDBUF", SO_SNDBUF as i64),
        ("SO_RCVBUF", SO_RCVBUF as i64),
        ("SO_SNDTIMEO", SO_SNDTIMEO as i64),
        ("SO_RCVTIMEO", SO_RCVTIMEO as i64),
        ("MSG_PEEK", MSG_PEEK as i64),
        ("MSG_WAITALL", MSG_WAITALL as i64),
        ("MSG_DONTWAIT", MSG_DONTWAIT as i64),
        ("IPPROTO_IP", IPPROTO_IP as i64),
        ("IPPROTO_TCP", IPPROTO_TCP as i64),
        ("IPPROTO_UDP", IPPROTO_UDP as i64),
        ("INADDR_ANY", INADDR_ANY as i64),
        ("TCP_NODELAY", TCP_NODELAY as i64),
        ("TCP_MAXSEG", TCP_MAXSEG as i64),
        ("EINTR", EINTR as i64),
        ("EAGAIN", EAGAIN as i64),
        ("EWOULDBLOCK", EWOULDBLOCK as i64),
        ("EINPROGRESS", EINPROGRESS as i64),
        ("ETIMEDOUT", ETIMEDOUT as i64),
        ("ECONNREFUSED", ECONNREFUSED as i64),
        ("SBTF_VAL", SBTF_VAL as i64),
        ("SBTF_REF", SBTF_REF as i64),
        ("SBTB_CODE", SBTB_CODE as i64),
        ("SBTS_CODE", SBTS_CODE as i64),
        ("SBTF_GET", SBTF_GET as i64),
        ("SBTF_SET", SBTF_SET as i64),
        ("SBTC_BREAKMASK", SBTC_BREAKMASK as i64),
        ("SBTC_HAVE_INTERFACE_API", SBTC_HAVE_INTERFACE_API as i64),
        ("SBTC_HAVE_STATUS_API", SBTC_HAVE_STATUS_API as i64),
        ("SM_Down", SM_Down as i64),
        ("SM_Up", SM_Up as i64),
        ("IFQ_BASE", IFQ_BASE as i64),
        ("IFQ_DeviceName", IFQ_DeviceName as i64),
        ("IFQ_DeviceUnit", IFQ_DeviceUnit as i64),
        ("IFQ_HardwareAddressSize", IFQ_HardwareAddressSize as i64),
        ("IFQ_HardwareAddress", IFQ_HardwareAddress as i64),
        ("IFQ_MTU", IFQ_MTU as i64),
        ("IFQ_BPS", IFQ_BPS as i64),
        ("IFQ_HardwareType", IFQ_HardwareType as i64),
        ("IFQ_PacketsReceived", IFQ_PacketsReceived as i64),
        ("IFQ_PacketsSent", IFQ_PacketsSent as i64),
        ("IFQ_BadData", IFQ_BadData as i64),
        ("IFQ_Overruns", IFQ_Overruns as i64),
        ("IFQ_UnknownTypes", IFQ_UnknownTypes as i64),
        ("IFQ_LastStart", IFQ_LastStart as i64),
        ("IFQ_Address", IFQ_Address as i64),
        ("IFQ_DestinationAddress", IFQ_DestinationAddress as i64),
        ("IFQ_BroadcastAddress", IFQ_BroadcastAddress as i64),
        ("IFQ_NetMask", IFQ_NetMask as i64),
        ("IFQ_Metric", IFQ_Metric as i64),
        ("IFQ_State", IFQ_State as i64),
        ("IFQ_AddressBindType", IFQ_AddressBindType as i64),
        ("IFQ_AddressLeaseExpires", IFQ_AddressLeaseExpires as i64),
        ("IFQ_PrimaryDNSAddress", IFQ_PrimaryDNSAddress as i64),
        ("IFQ_SecondaryDNSAddress", IFQ_SecondaryDNSAddress as i64),
        ("IFQ_NumReadRequests", IFQ_NumReadRequests as i64),
        ("IFQ_MaxReadRequests", IFQ_MaxReadRequests as i64),
        ("IFQ_NumWriteRequests", IFQ_NumWriteRequests as i64),
        ("IFQ_MaxWriteRequests", IFQ_MaxWriteRequests as i64),
        ("IFQ_GetBytesIn", IFQ_GetBytesIn as i64),
        ("IFQ_GetBytesOut", IFQ_GetBytesOut as i64),
        ("IFQ_GetDebugMode", IFQ_GetDebugMode as i64),
        ("IFQ_GetSANA2CopyStats", IFQ_GetSANA2CopyStats as i64),
        ("IFQ_NumReadRequestsPending", IFQ_NumReadRequestsPending as i64),
        ("IFQ_NumWriteRequestsPending", IFQ_NumWriteRequestsPending as i64),
        ("IFQ_HardwareMTU", IFQ_HardwareMTU as i64),
        ("IFQ_OutputDrops", IFQ_OutputDrops as i64),
        ("IFQ_InputDrops", IFQ_InputDrops as i64),
        ("IFQ_OutputErrors", IFQ_OutputErrors as i64),
        ("IFQ_InputErrors", IFQ_InputErrors as i64),
        ("IFQ_OutputMulticasts", IFQ_OutputMulticasts as i64),
        ("IFQ_InputMulticasts", IFQ_InputMulticasts as i64),
        ("IFQ_IPDrops", IFQ_IPDrops as i64),
        ("IFQ_ARPDrops", IFQ_ARPDrops as i64),
        ("NETWORKSTATUS_VERSION", NETWORKSTATUS_VERSION as i64),
        ("NETSTATUS_icmp", NETSTATUS_icmp as i64),
        ("NETSTATUS_igmp", NETSTATUS_igmp as i64),
        ("NETSTATUS_ip", NETSTATUS_ip as i64),
        ("NETSTATUS_mb", NETSTATUS_mb as i64),
        ("NETSTATUS_mrt", NETSTATUS_mrt as i64),
        ("NETSTATUS_rt", NETSTATUS_rt as i64),
        ("NETSTATUS_tcp", NETSTATUS_tcp as i64),
        ("NETSTATUS_udp", NETSTATUS_udp as i64),
        ("NETSTATUS_tcp_sockets", NETSTATUS_tcp_sockets as i64),
        ("NETSTATUS_udp_sockets", NETSTATUS_udp_sockets as i64),
    ];
    let mut bad = Vec::new();
    for &(name, ours) in &checks {
        match definition(&texts, name).and_then(|d| eval(&texts, d, 0)) {
            Some(v) if v == ours => {}
            Some(v) => bad.push(format!("{}: SDK {}, crate {}", name, v, ours)),
            None => bad.push(format!("{}: not found or not evaluated in the SDK", name)),
        }
    }
    assert!(bad.is_empty(), "bsdsocket constants differ from the SDK:\n{}", bad.join("\n"));
    eprintln!("[ok] {} bsdsocket constants match the SDK", checks.len());
}

#[test]
fn statistics_tables_match_the_sdk_structs() {
    let Some(net) = netinclude() else {
        eprintln!("[skip] AmigaOS 4 SDK netinclude not found (set AMIGA_SDK to override).");
        return;
    };
    let cases: [(&str, &str, &[&str]); 4] = [
        ("netinet/tcp_var.h", "tcpstat", TCPSTAT_FIELDS),
        ("netinet/ip_var.h", "ipstat", IPSTAT_FIELDS),
        ("netinet/udp_var.h", "udpstat", UDPSTAT_FIELDS),
        ("sys/mbuf.h", "mbstat", MBSTAT_FIELDS),
    ];
    for (file, name, ours) in cases {
        let sdk = struct_fields(&read(&net, file), name);
        assert!(!sdk.is_empty(), "struct {} in {}: no fields parsed", name, file);
        assert_eq!(sdk, ours, "struct {} ({}) differs from the crate's table", name, file);
    }
    let copy = struct_fields(&read(&net, "libraries/bsdsocket.h"), "SANA2CopyStats");
    assert_eq!(copy, ["s2cs_DMAIn", "s2cs_DMAOut", "s2cs_ByteIn", "s2cs_ByteOut", "s2cs_WordOut"]);
    let pcd = struct_fields(&read(&net, "libraries/bsdsocket.h"), "protocol_connection_data");
    assert_eq!(pcd, ["pcd_foreign_address", "pcd_foreign_port", "pcd_local_address", "pcd_local_port",
                     "pcd_receive_queue_size", "pcd_send_queue_size", "pcd_tcp_state"]);
    eprintln!("[ok] tcpstat, ipstat, udpstat, mbstat, SANA2CopyStats and protocol_connection_data match");
}

#[test]
fn socket_base_tag_macros() {
    // SBTM_GETREF(code) = TAG_USER | SBTF_REF | (code << SBTB_CODE) | SBTF_GET
    assert_eq!(sbtm_getref(SBTC_HAVE_STATUS_API), 0x8000_0000 | 0x8000 | (53 << 1));
    assert_eq!(sbtm_setval(SBTC_BREAKMASK), 0x8000_0000 | (1 << 1) | 1);
}

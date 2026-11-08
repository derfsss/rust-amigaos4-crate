//! amigaos4::sana2's constants against the SDK's devices/sana2.h (with
//! exec/io.h, utility/tagitem.h and devices/newstyle.h for the names it
//! builds on). Skips, saying so, without an SDK at `sdk/` or `AMIGA_SDK`.

#![cfg(feature = "sana2")]

use std::fs;
mod common;
use common::{definition, eval, include_dir};

use amigaos4::sana2::*;

#[test]
fn sana2_constants_match_the_sdk() {
    let Some(inc) = include_dir() else {
        eprintln!("[skip] AmigaOS 4 SDK not found at sdk/ (set AMIGA_SDK to override).");
        return;
    };
    let read = |p: &str| fs::read_to_string(inc.join(p)).unwrap_or_else(|e| panic!("{}: {}", p, e));
    let sana2 = read("devices/sana2.h");
    let io = read("exec/io.h");
    let tagitem = read("utility/tagitem.h");
    let newstyle = read("devices/newstyle.h");
    let texts = [sana2.as_str(), io.as_str(), tagitem.as_str(), newstyle.as_str()];

    let checks: Vec<(&str, i64)> = vec![
        ("SANA2_MAX_ADDR_BITS", SANA2_MAX_ADDR_BITS as i64),
        ("SANA2IOB_RAW", SANA2IOB_RAW as i64),
        ("SANA2IOB_BCAST", SANA2IOB_BCAST as i64),
        ("SANA2IOB_MCAST", SANA2IOB_MCAST as i64),
        ("SANA2IOB_QUICK", SANA2IOB_QUICK as i64),
        ("SANA2OPB_MINE", SANA2OPB_MINE as i64),
        ("SANA2OPB_PROM", SANA2OPB_PROM as i64),
        ("S2_Dummy", S2_DUMMY as i64),
        ("S2_CopyToBuff", S2_COPY_TO_BUFF as i64),
        ("S2_CopyFromBuff", S2_COPY_FROM_BUFF as i64),
        ("S2_PacketFilter", S2_PACKET_FILTER as i64),
        ("S2_CopyToBuff16", S2_COPY_TO_BUFF16 as i64),
        ("S2_CopyFromBuff16", S2_COPY_FROM_BUFF16 as i64),
        ("S2_CopyToBuff32", S2_COPY_TO_BUFF32 as i64),
        ("S2_CopyFromBuff32", S2_COPY_FROM_BUFF32 as i64),
        ("S2_DMACopyToBuff32", S2_DMA_COPY_TO_BUFF32 as i64),
        ("S2_DMACopyFromBuff32", S2_DMA_COPY_FROM_BUFF32 as i64),
        ("S2_DMACopyToBuff64", S2_DMA_COPY_TO_BUFF64 as i64),
        ("S2_DMACopyFromBuff64", S2_DMA_COPY_FROM_BUFF64 as i64),
        ("S2_Log", S2_LOG as i64),
        ("S2WireType_Ethernet", S2_WIRETYPE_ETHERNET as i64),
        ("S2WireType_IEEE802", S2_WIRETYPE_IEEE802 as i64),
        ("S2WireType_Arcnet", S2_WIRETYPE_ARCNET as i64),
        ("S2WireType_LocalTalk", S2_WIRETYPE_LOCALTALK as i64),
        ("S2WireType_DyLAN", S2_WIRETYPE_DYLAN as i64),
        ("S2WireType_AmokNet", S2_WIRETYPE_AMOKNET as i64),
        ("S2WireType_Liana", S2_WIRETYPE_LIANA as i64),
        ("S2WireType_PPP", S2_WIRETYPE_PPP as i64),
        ("S2WireType_SLIP", S2_WIRETYPE_SLIP as i64),
        ("S2WireType_CSLIP", S2_WIRETYPE_CSLIP as i64),
        ("S2WireType_PLIP", S2_WIRETYPE_PLIP as i64),
        ("S2LOG_Emergency", S2LOG_EMERGENCY as i64),
        ("S2LOG_Alert", S2LOG_ALERT as i64),
        ("S2LOG_Critical", S2LOG_CRITICAL as i64),
        ("S2LOG_Error", S2LOG_ERROR as i64),
        ("S2LOG_Warning", S2LOG_WARNING as i64),
        ("S2LOG_Notice", S2LOG_NOTICE as i64),
        ("S2LOG_Information", S2LOG_INFORMATION as i64),
        ("S2LOG_Debug", S2LOG_DEBUG as i64),
        ("S2_START", S2_START as i64),
        ("S2_DEVICEQUERY", S2_DEVICEQUERY as i64),
        ("S2_GETSTATIONADDRESS", S2_GETSTATIONADDRESS as i64),
        ("S2_CONFIGINTERFACE", S2_CONFIGINTERFACE as i64),
        ("S2_ADDMULTICASTADDRESS", S2_ADDMULTICASTADDRESS as i64),
        ("S2_DELMULTICASTADDRESS", S2_DELMULTICASTADDRESS as i64),
        ("S2_MULTICAST", S2_MULTICAST as i64),
        ("S2_BROADCAST", S2_BROADCAST as i64),
        ("S2_TRACKTYPE", S2_TRACKTYPE as i64),
        ("S2_UNTRACKTYPE", S2_UNTRACKTYPE as i64),
        ("S2_GETTYPESTATS", S2_GETTYPESTATS as i64),
        ("S2_GETSPECIALSTATS", S2_GETSPECIALSTATS as i64),
        ("S2_GETGLOBALSTATS", S2_GETGLOBALSTATS as i64),
        ("S2_ONEVENT", S2_ONEVENT as i64),
        ("S2_READORPHAN", S2_READORPHAN as i64),
        ("S2_ONLINE", S2_ONLINE as i64),
        ("S2_OFFLINE", S2_OFFLINE as i64),
        ("S2_END", S2_END as i64),
        ("S2_ADDMULTICASTADDRESSES", S2_ADDMULTICASTADDRESSES as i64),
        ("S2_DELMULTICASTADDRESSES", S2_DELMULTICASTADDRESSES as i64),
        ("S2_GETPEERADDRESS", S2_GETPEERADDRESS as i64),
        ("S2_GETDNSADDRESS", S2_GETDNSADDRESS as i64),
        ("S2_GETEXTENDEDGLOBALSTATS", S2_GETEXTENDEDGLOBALSTATS as i64),
        ("S2_CONNECT", S2_CONNECT as i64),
        ("S2_DISCONNECT", S2_DISCONNECT as i64),
        ("S2_SAMPLE_THROUGHPUT", S2_SAMPLE_THROUGHPUT as i64),
        ("S2_SANA2HOOK", S2_SANA2HOOK as i64),
        ("NSDEVTYPE_SANA2", NSDEVTYPE_SANA2 as i64),
        ("S2ERR_NO_ERROR", S2ERR_NO_ERROR as i64),
        ("S2ERR_NO_RESOURCES", S2ERR_NO_RESOURCES as i64),
        ("S2ERR_BAD_ARGUMENT", S2ERR_BAD_ARGUMENT as i64),
        ("S2ERR_BAD_STATE", S2ERR_BAD_STATE as i64),
        ("S2ERR_BAD_ADDRESS", S2ERR_BAD_ADDRESS as i64),
        ("S2ERR_MTU_EXCEEDED", S2ERR_MTU_EXCEEDED as i64),
        ("S2ERR_NOT_SUPPORTED", S2ERR_NOT_SUPPORTED as i64),
        ("S2ERR_SOFTWARE", S2ERR_SOFTWARE as i64),
        ("S2ERR_OUTOFSERVICE", S2ERR_OUTOFSERVICE as i64),
        ("S2ERR_TX_FAILURE", S2ERR_TX_FAILURE as i64),
        ("S2WERR_GENERIC_ERROR", S2WERR_GENERIC_ERROR as i64),
        ("S2WERR_NOT_CONFIGURED", S2WERR_NOT_CONFIGURED as i64),
        ("S2WERR_UNIT_ONLINE", S2WERR_UNIT_ONLINE as i64),
        ("S2WERR_UNIT_OFFLINE", S2WERR_UNIT_OFFLINE as i64),
        ("S2WERR_ALREADY_TRACKED", S2WERR_ALREADY_TRACKED as i64),
        ("S2WERR_NOT_TRACKED", S2WERR_NOT_TRACKED as i64),
        ("S2WERR_BUFF_ERROR", S2WERR_BUFF_ERROR as i64),
        ("S2WERR_SRC_ADDRESS", S2WERR_SRC_ADDRESS as i64),
        ("S2WERR_DST_ADDRESS", S2WERR_DST_ADDRESS as i64),
        ("S2WERR_BAD_BROADCAST", S2WERR_BAD_BROADCAST as i64),
        ("S2WERR_BAD_MULTICAST", S2WERR_BAD_MULTICAST as i64),
        ("S2WERR_MULTICAST_FULL", S2WERR_MULTICAST_FULL as i64),
        ("S2WERR_BAD_EVENT", S2WERR_BAD_EVENT as i64),
        ("S2WERR_BAD_STATDATA", S2WERR_BAD_STATDATA as i64),
        ("S2WERR_IS_CONFIGURED", S2WERR_IS_CONFIGURED as i64),
        ("S2WERR_NULL_POINTER", S2WERR_NULL_POINTER as i64),
        ("S2WERR_TOO_MANY_RETRIES", S2WERR_TOO_MANY_RETRIES as i64),
        ("S2WERR_RCVREL_HDW_ERR", S2WERR_RCVREL_HDW_ERR as i64),
        ("S2WERR_UNIT_DISCONNECTED", S2WERR_UNIT_DISCONNECTED as i64),
        ("S2WERR_UNIT_CONNECTED", S2WERR_UNIT_CONNECTED as i64),
        ("S2WERR_INVALID_OPTION", S2WERR_INVALID_OPTION as i64),
        ("S2WERR_MISSING_OPTION", S2WERR_MISSING_OPTION as i64),
        ("S2WERR_AUTHENTICATION_FAILED", S2WERR_AUTHENTICATION_FAILED as i64),
        ("S2WERR_FUNCTIONS_MISSING", S2WERR_FUNCTIONS_MISSING as i64),
        ("S2EVENT_ERROR", S2EVENT_ERROR as i64),
        ("S2EVENT_TX", S2EVENT_TX as i64),
        ("S2EVENT_RX", S2EVENT_RX as i64),
        ("S2EVENT_ONLINE", S2EVENT_ONLINE as i64),
        ("S2EVENT_OFFLINE", S2EVENT_OFFLINE as i64),
        ("S2EVENT_BUFF", S2EVENT_BUFF as i64),
        ("S2EVENT_HARDWARE", S2EVENT_HARDWARE as i64),
        ("S2EVENT_SOFTWARE", S2EVENT_SOFTWARE as i64),
        ("S2EVENT_CONFIGCHANGED", S2EVENT_CONFIGCHANGED as i64),
        ("S2EVENT_CONNECT", S2EVENT_CONNECT as i64),
        ("S2EVENT_DISCONNECT", S2EVENT_DISCONNECT as i64),
    ];
    let mut bad = Vec::new();
    for &(name, ours) in &checks {
        match definition(&texts, name).and_then(|d| eval(&texts, d, 0)) {
            Some(v) if v == ours => {}
            Some(v) => bad.push(format!("{}: SDK {}, crate {}", name, v, ours)),
            None => bad.push(format!("{}: not found or not evaluated in the SDK", name)),
        }
    }
    assert!(bad.is_empty(), "sana2 constants differ from the SDK:\n{}", bad.join("\n"));
    eprintln!("[ok] {} sana2 constants match the SDK", checks.len());
}

#[test]
fn evaluator_handles_the_headers_forms() {
    let t = "#define A (TAG_USER + 0xB0000)\n#define TAG_USER ((ULONG)(1UL<<31))\n#define B (A +  1)\n    C = 9,\n#define D (C+ 2)\n#define E (1UL<< 4)\n";
    let texts = [t];
    // (ULONG) casts are not handled; TAG_USER is checked through the real header.
    assert_eq!(eval(&texts, "(1UL<< 4)", 0), Some(16));
    assert_eq!(eval(&texts, definition(&texts, "D").unwrap(), 0), Some(11));
    assert_eq!(eval(&texts, definition(&texts, "E").unwrap(), 0), Some(16));
    assert_eq!(eval(&texts, "0xC008", 0), Some(0xC008));
}

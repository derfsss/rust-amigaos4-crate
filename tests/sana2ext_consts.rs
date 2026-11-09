//! amigaos4::sana2ext's constants against the crate's own
//! include/devices/sana2ext.h (no SDK needed).

#![cfg(feature = "sana2")]

use std::fs;
mod common;
use common::eval;

use amigaos4::sana2ext::*;

#[test]
fn sana2ext_constants_match_the_header() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("include/devices/sana2ext.h");
    let header = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
    let texts = [header.as_str()];
    let checks: Vec<(&str, i64)> = vec![
        ("S2X_MAGIC", S2X_MAGIC as i64),
        ("S2X_VERSION", S2X_VERSION as i64),
        ("S2X_QUERY", S2X_QUERY as i64),
        ("S2X_READBATCH", S2X_READBATCH as i64),
        ("S2X_WRITEBATCH", S2X_WRITEBATCH as i64),
        ("S2XCAP_BATCH", S2XCAP_BATCH as i64),
        ("S2XCAP_RXCHECKSUM", S2XCAP_RXCHECKSUM as i64),
        ("S2XCAP_TXCHECKSUM", S2XCAP_TXCHECKSUM as i64),
        ("S2XCAP_SEGMENT", S2XCAP_SEGMENT as i64),
        ("S2XFRAME_CHECKSUM", S2XFRAME_CHECKSUM as i64),
        ("S2XFRAME_SEGMENT", S2XFRAME_SEGMENT as i64),
        ("S2XITEM_PRIVATE", S2XITEM_PRIVATE as i64),
        ("S2XF_BATCH", S2XF_BATCH as i64),
        ("S2XF_RXCHECKSUM", S2XF_RXCHECKSUM as i64),
        ("S2XF_TXCHECKSUM", S2XF_TXCHECKSUM as i64),
        ("S2XF_SEGMENT", S2XF_SEGMENT as i64),
        ("S2XCSUM_IPV4", S2XCSUM_IPV4 as i64),
        ("S2XCSUM_TCPV4", S2XCSUM_TCPV4 as i64),
        ("S2XCSUM_UDPV4", S2XCSUM_UDPV4 as i64),
        ("S2XCSUM_TCPV6", S2XCSUM_TCPV6 as i64),
        ("S2XCSUM_UDPV6", S2XCSUM_UDPV6 as i64),
        ("S2XCS_IP_GOOD", S2XCS_IP_GOOD as i64),
        ("S2XCS_IP_BAD", S2XCS_IP_BAD as i64),
        ("S2XCS_L4_GOOD", S2XCS_L4_GOOD as i64),
        ("S2XCS_L4_BAD", S2XCS_L4_BAD as i64),
        ("S2XCS_FILL_IP", S2XCS_FILL_IP as i64),
        ("S2XCS_FILL_L4", S2XCS_FILL_L4 as i64),
    ];
    let mut bad = Vec::new();
    for (name, rust) in &checks {
        match eval(&texts, name, 0) {
            Some(c) if c == *rust => {}
            Some(c) => bad.push(format!("{}: header {:#x}, Rust {:#x}", name, c, rust)),
            None => bad.push(format!("{}: not found or not evaluated in the header", name)),
        }
    }
    assert!(bad.is_empty(), "sana2ext constants differ:\n{}", bad.join("\n"));
}

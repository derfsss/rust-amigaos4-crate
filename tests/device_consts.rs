//! amigaos4::device's constants against the SDK headers (exec/io.h,
//! exec/errors.h, devices/newstyle.h). Skips, saying so, without an SDK at
//! `sdk/` or `AMIGA_SDK` (see tests/sdk_audit.rs).

#![cfg(feature = "device")]

use std::fs;
use std::path::PathBuf;

use amigaos4::device::{cmd, ioerr, td, tderr, IOF_QUICK, NSDEVTYPE_TRACKDISK, NSDEVTYPE_UNKNOWN};

fn include_dir() -> Option<PathBuf> {
    let root = std::env::var("AMIGA_SDK")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sdk"));
    let inc = root.join("include").join("include_h");
    if inc.is_dir() { Some(inc) } else { None }
}

/// The value the header gives `name`: `#define NAME value` or an enum entry
/// `NAME = value`. Values: integers (decimal or 0x hex, maybe in
/// parentheses, maybe negative) or `(1<<OTHER)` with OTHER defined.
fn value(text: &str, name: &str) -> Option<i64> {
    for line in text.lines() {
        let l = line.trim();
        let rest = if let Some(r) = l.strip_prefix("#define") {
            let r = r.trim_start();
            r.strip_prefix(name).filter(|r| r.starts_with(char::is_whitespace))
        } else {
            l.strip_prefix(name).map(|r| r.trim_start()).and_then(|r| r.strip_prefix('='))
        };
        let Some(rest) = rest else { continue };
        let v = rest.split("/*").next()?.trim().trim_end_matches(',').trim();
        let v = v.trim_start_matches('(').trim_end_matches(')').trim();
        for op in ['|', '+'] {
            if let Some((a, b)) = v.split_once(op) {
                let x = operand(text, a.trim())?;
                let y = operand(text, b.trim())?;
                return Some(if op == '|' { x | y } else { x + y });
            }
        }
        if let Some((a, b)) = v.split_once("<<") {
            let base: i64 = a.trim().parse().ok()?;
            let shift = value(text, b.trim()).or_else(|| b.trim().parse().ok())?;
            return Some(base << shift);
        }
        let v = v.replace(' ', "");
        return if let Some(h) = v.strip_prefix("0x") {
            i64::from_str_radix(h, 16).ok()
        } else {
            v.parse().ok()
        };
    }
    None
}

/// A number, or a name the header defines.
fn operand(text: &str, s: &str) -> Option<i64> {
    let s = s.trim_start_matches('(').trim_end_matches(')').trim();
    if let Some(h) = s.strip_prefix("0x") {
        return i64::from_str_radix(h, 16).ok();
    }
    s.parse().ok().or_else(|| value(text, s))
}

#[test]
fn device_constants_match_the_sdk() {
    let Some(inc) = include_dir() else {
        eprintln!("[skip] AmigaOS 4 SDK not found at sdk/ (set AMIGA_SDK to override).");
        return;
    };
    let read = |p: &str| fs::read_to_string(inc.join(p)).unwrap_or_else(|e| panic!("{}: {}", p, e));
    let io = read("exec/io.h");
    let errors = read("exec/errors.h");
    let newstyle = read("devices/newstyle.h");
    // trackdisk.h's TD_ commands are CMD_NONSTD+n and its ETD_ ones
    // CMD_x|TDF_EXTCOM: give the parser exec/io.h's names too.
    let trackdisk = format!("{}
{}", read("devices/trackdisk.h"), io);
    let scsidisk = read("devices/scsidisk.h");
    let mounter = read("libraries/mounter.h");

    let checks: &[(&str, &str, i64)] = &[
        (&io, "CMD_INVALID", cmd::CMD_INVALID as i64),
        (&io, "CMD_RESET", cmd::CMD_RESET as i64),
        (&io, "CMD_READ", cmd::CMD_READ as i64),
        (&io, "CMD_WRITE", cmd::CMD_WRITE as i64),
        (&io, "CMD_UPDATE", cmd::CMD_UPDATE as i64),
        (&io, "CMD_CLEAR", cmd::CMD_CLEAR as i64),
        (&io, "CMD_STOP", cmd::CMD_STOP as i64),
        (&io, "CMD_START", cmd::CMD_START as i64),
        (&io, "CMD_FLUSH", cmd::CMD_FLUSH as i64),
        (&io, "CMD_NONSTD", cmd::CMD_NONSTD as i64),
        (&io, "IOF_QUICK", IOF_QUICK as i64),
        (&errors, "IOERR_SUCCESS", ioerr::IOERR_SUCCESS as i64),
        (&errors, "IOERR_OPENFAIL", ioerr::IOERR_OPENFAIL as i64),
        (&errors, "IOERR_ABORTED", ioerr::IOERR_ABORTED as i64),
        (&errors, "IOERR_NOCMD", ioerr::IOERR_NOCMD as i64),
        (&errors, "IOERR_BADLENGTH", ioerr::IOERR_BADLENGTH as i64),
        (&errors, "IOERR_BADADDRESS", ioerr::IOERR_BADADDRESS as i64),
        (&errors, "IOERR_UNITBUSY", ioerr::IOERR_UNITBUSY as i64),
        (&errors, "IOERR_SELFTEST", ioerr::IOERR_SELFTEST as i64),
        (&newstyle, "NSCMD_DEVICEQUERY", cmd::NSCMD_DEVICEQUERY as i64),
        (&newstyle, "NSDEVTYPE_UNKNOWN", NSDEVTYPE_UNKNOWN as i64),
        (&newstyle, "NSDEVTYPE_TRACKDISK", NSDEVTYPE_TRACKDISK as i64),
        (&trackdisk, "TDF_EXTCOM", td::TDF_EXTCOM as i64),
        (&trackdisk, "TD_MOTOR", td::TD_MOTOR as i64),
        (&trackdisk, "TD_SEEK", td::TD_SEEK as i64),
        (&trackdisk, "TD_FORMAT", td::TD_FORMAT as i64),
        (&trackdisk, "TD_REMOVE", td::TD_REMOVE as i64),
        (&trackdisk, "TD_CHANGENUM", td::TD_CHANGENUM as i64),
        (&trackdisk, "TD_CHANGESTATE", td::TD_CHANGESTATE as i64),
        (&trackdisk, "TD_PROTSTATUS", td::TD_PROTSTATUS as i64),
        (&trackdisk, "TD_RAWREAD", td::TD_RAWREAD as i64),
        (&trackdisk, "TD_RAWWRITE", td::TD_RAWWRITE as i64),
        (&trackdisk, "TD_GETDRIVETYPE", td::TD_GETDRIVETYPE as i64),
        (&trackdisk, "TD_GETNUMTRACKS", td::TD_GETNUMTRACKS as i64),
        (&trackdisk, "TD_ADDCHANGEINT", td::TD_ADDCHANGEINT as i64),
        (&trackdisk, "TD_REMCHANGEINT", td::TD_REMCHANGEINT as i64),
        (&trackdisk, "TD_GETGEOMETRY", td::TD_GETGEOMETRY as i64),
        (&trackdisk, "TD_EJECT", td::TD_EJECT as i64),
        (&trackdisk, "TD_READ64", td::TD_READ64 as i64),
        (&trackdisk, "TD_WRITE64", td::TD_WRITE64 as i64),
        (&trackdisk, "TD_SEEK64", td::TD_SEEK64 as i64),
        (&trackdisk, "TD_FORMAT64", td::TD_FORMAT64 as i64),
        (&trackdisk, "ETD_WRITE", td::ETD_WRITE as i64),
        (&trackdisk, "ETD_READ", td::ETD_READ as i64),
        (&trackdisk, "ETD_MOTOR", td::ETD_MOTOR as i64),
        (&trackdisk, "ETD_SEEK", td::ETD_SEEK as i64),
        (&trackdisk, "ETD_FORMAT", td::ETD_FORMAT as i64),
        (&trackdisk, "ETD_UPDATE", td::ETD_UPDATE as i64),
        (&trackdisk, "ETD_CLEAR", td::ETD_CLEAR as i64),
        (&trackdisk, "DRIVE3_5", td::DRIVE3_5 as i64),
        (&newstyle, "NSCMD_TD_READ64", td::NSCMD_TD_READ64 as i64),
        (&newstyle, "NSCMD_TD_WRITE64", td::NSCMD_TD_WRITE64 as i64),
        (&newstyle, "NSCMD_TD_SEEK64", td::NSCMD_TD_SEEK64 as i64),
        (&newstyle, "NSCMD_TD_FORMAT64", td::NSCMD_TD_FORMAT64 as i64),
        (&newstyle, "NSCMD_ETD_READ64", td::NSCMD_ETD_READ64 as i64),
        (&newstyle, "NSCMD_ETD_WRITE64", td::NSCMD_ETD_WRITE64 as i64),
        (&newstyle, "NSCMD_ETD_SEEK64", td::NSCMD_ETD_SEEK64 as i64),
        (&newstyle, "NSCMD_ETD_FORMAT64", td::NSCMD_ETD_FORMAT64 as i64),
        (&mounter, "NSCMD_TD_GETGEOMETRY64", td::NSCMD_TD_GETGEOMETRY64 as i64),
        (&scsidisk, "HD_SCSICMD", td::HD_SCSICMD as i64),
        (&trackdisk, "TDERR_NotSpecified", tderr::TDERR_NOT_SPECIFIED as i64),
        (&trackdisk, "TDERR_NoSecHdr", tderr::TDERR_NO_SEC_HDR as i64),
        (&trackdisk, "TDERR_BadSecHdr", tderr::TDERR_BAD_SEC_HDR as i64),
        (&trackdisk, "TDERR_WriteProt", tderr::TDERR_WRITE_PROT as i64),
        (&trackdisk, "TDERR_DiskChanged", tderr::TDERR_DISK_CHANGED as i64),
        (&trackdisk, "TDERR_SeekError", tderr::TDERR_SEEK_ERROR as i64),
        (&trackdisk, "TDERR_NoMem", tderr::TDERR_NO_MEM as i64),
        (&trackdisk, "TDERR_BadUnitNum", tderr::TDERR_BAD_UNIT_NUM as i64),
        (&trackdisk, "TDERR_BadDriveType", tderr::TDERR_BAD_DRIVE_TYPE as i64),
        (&scsidisk, "HFERR_SelfUnit", tderr::HFERR_SELF_UNIT as i64),
        (&scsidisk, "HFERR_DMA", tderr::HFERR_DMA as i64),
        (&scsidisk, "HFERR_Phase", tderr::HFERR_PHASE as i64),
        (&scsidisk, "HFERR_SelTimeout", tderr::HFERR_SEL_TIMEOUT as i64),
        (&scsidisk, "HFERR_BadStatus", tderr::HFERR_BAD_STATUS as i64),
    ];
    let mut bad = Vec::new();
    for &(text, name, ours) in checks {
        match value(text, name) {
            Some(v) if v == ours => {}
            Some(v) => bad.push(format!("{}: SDK {} , crate {}", name, v, ours)),
            None => bad.push(format!("{}: not found in the SDK", name)),
        }
    }
    assert!(bad.is_empty(), "device constants differ from the SDK:\n{}", bad.join("\n"));
    eprintln!("[ok] {} device constants match the SDK", checks.len());
}

#[test]
fn value_parser_reads_the_forms_the_headers_use() {
    let t = "#define A 0x4000\n    B   =   (-3), /* x */\n#define C (1<<D)\n#define D 2\n    E = 9\n";
    assert_eq!(value(t, "A"), Some(0x4000));
    assert_eq!(value(t, "B"), Some(-3));
    assert_eq!(value(t, "C"), Some(4));
    assert_eq!(value(t, "E"), Some(9));
    assert_eq!(value(t, "F"), None);
}

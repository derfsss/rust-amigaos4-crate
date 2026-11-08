//! The utility.library constants the `utility` module uses, against the
//! SDK's utility/*.h. Skips, saying so, without an SDK at `sdk/` or
//! `AMIGA_SDK`.

use std::fs;
mod common;
use common::{definition, eval, include_dir};

use amigaos4::sys;

#[test]
fn utility_constants_match_the_sdk() {
    let Some(inc) = include_dir() else {
        eprintln!("[skip] AmigaOS 4 SDK not found at sdk/ (set AMIGA_SDK to override).");
        return;
    };
    let read = |p: &str| fs::read_to_string(inc.join(p)).unwrap_or_else(|e| panic!("{}: {}", p, e));
    let tagitem = read("utility/tagitem.h");
    let name = read("utility/name.h");
    let utility = read("utility/utility.h");
    let texts = [tagitem.as_str(), name.as_str(), utility.as_str()];

    let checks: Vec<(&str, i64)> = vec![
        ("TAG_DONE", sys::TAG_DONE as i64),
        ("TAG_IGNORE", sys::TAG_IGNORE as i64),
        ("TAG_MORE", sys::TAG_MORE as i64),
        ("TAG_SKIP", sys::TAG_SKIP as i64),
        ("TAGFILTER_AND", sys::TAGFILTER_AND as i64),
        ("TAGFILTER_NOT", sys::TAGFILTER_NOT as i64),
        ("MAP_REMOVE_NOT_FOUND", sys::MAP_REMOVE_NOT_FOUND as i64),
        ("MAP_KEEP_NOT_FOUND", sys::MAP_KEEP_NOT_FOUND as i64),
        ("ANO_NameSpace", sys::ANO_NAMESPACE as i64),
        ("ANO_UserSpace", sys::ANO_USERSPACE as i64),
        ("ANO_Priority", sys::ANO_PRIORITY as i64),
        ("ANO_Flags", sys::ANO_FLAGS as i64),
        ("NSF_NODUPS", sys::NSF_NODUPS as i64),
        ("NSF_CASE", sys::NSF_CASE as i64),
        ("UTF_INVALID_SUBST_FFFD", sys::UTF_INVALID_SUBST_FFFD as i64),
    ];
    let mut bad = Vec::new();
    for (name, ours) in &checks {
        let def = definition(&texts, name).unwrap_or_else(|| panic!("{} not found in the SDK", name));
        let sdk = eval(&texts, def, 0).unwrap_or_else(|| panic!("{}: cannot evaluate {:?}", name, def));
        if sdk != *ours {
            bad.push(format!("{}: SDK {} ours {}", name, sdk, ours));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
    assert!(checks.len() >= 15, "the check list was emptied");
}

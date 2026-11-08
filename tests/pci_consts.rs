//! The PCI and GetMachineInfo constants the `pci` and `expansion` modules
//! use, against the SDK's expansion/pci.h and expansion/expansion.h.
//! Skips, saying so, without an SDK at `sdk/` or `AMIGA_SDK`.

use std::fs;
mod common;
use common::{definition, eval, include_dir};

use amigaos4::sys;

#[test]
fn pci_constants_match_the_sdk() {
    let Some(inc) = include_dir() else {
        eprintln!("[skip] AmigaOS 4 SDK not found at sdk/ (set AMIGA_SDK to override).");
        return;
    };
    let read = |p: &str| fs::read_to_string(inc.join(p)).unwrap_or_else(|e| panic!("{}: {}", p, e));
    let pci = read("expansion/pci.h");
    let exp = read("expansion/expansion.h");
    let tagitem = read("utility/tagitem.h");
    let texts = [pci.as_str(), exp.as_str(), tagitem.as_str()];

    let checks: Vec<(&str, i64)> = vec![
        ("FDT_VendorID", sys::FDT_VENDOR_ID as i64),
        ("FDT_DeviceID", sys::FDT_DEVICE_ID as i64),
        ("FDT_Index", sys::FDT_INDEX as i64),
        ("FDT_Class", sys::FDT_CLASS as i64),
        ("FDT_ClassMask", sys::FDT_CLASS_MASK as i64),
        ("FDT_CandidateList", sys::FDT_CANDIDATE_LIST as i64),
        ("FDT_BusNr", sys::FDT_BUS_NR as i64),
        ("FDT_DeviceNr", sys::FDT_DEVICE_NR as i64),
        ("FDT_FunctionNr", sys::FDT_FUNCTION_NR as i64),
        ("PCI_RANGE_IO", sys::PCI_RANGE_IO as i64),
        ("PCI_RANGE_MEMORY", sys::PCI_RANGE_MEMORY as i64),
        ("PCI_RANGE_PREFETCH", sys::PCI_RANGE_PREFETCH as i64),
        ("PCI_NUM_RESOURCE_RANGES", sys::PCI_NUM_RESOURCE_RANGES as i64),
        ("PCI_LOCK_SHARED", sys::PCI_LOCK_SHARED as i64),
        ("PCI_LOCK_EXCLUSIVE", sys::PCI_LOCK_EXCLUSIVE as i64),
        ("PCI_CAP_BUSMASTER", sys::PCI_CAP_BUSMASTER as i64),
        ("PCI_CAP_INTERCARD", sys::PCI_CAP_INTERCARD as i64),
        ("PCI_CAP_CONFIGME", sys::PCI_CAP_CONFIGME as i64),
        ("PCI_CAP_BRIDGE", sys::PCI_CAP_BRIDGE as i64),
        ("PCI_CAP_CARDBUS", sys::PCI_CAP_CARDBUS as i64),
        ("GMIT_Machine", sys::GMIT_MACHINE as i64),
        ("GMIT_MachineString", sys::GMIT_MACHINE_STRING as i64),
        ("GMIT_MemoryAvailable", sys::GMIT_MEMORY_AVAILABLE as i64),
        ("GMIT_Extensions", sys::GMIT_EXTENSIONS as i64),
        ("GMIT_TotalPhysicalMemory", sys::GMIT_TOTAL_PHYSICAL_MEMORY as i64),
        ("MACHINETYPE_UNKNOWN", sys::MACHINETYPE_UNKNOWN as i64),
        ("MACHINETYPE_BLIZZARDPPC", sys::MACHINETYPE_BLIZZARDPPC as i64),
        ("MACHINETYPE_CYBERSTORMPPC", sys::MACHINETYPE_CYBERSTORMPPC as i64),
        ("MACHINETYPE_AMIGAONE", sys::MACHINETYPE_AMIGAONE as i64),
        ("MACHINETYPE_SAM440", sys::MACHINETYPE_SAM440 as i64),
        ("MACHINETYPE_PEGASOS2", sys::MACHINETYPE_PEGASOS2 as i64),
        ("MACHINETYPE_X1000", sys::MACHINETYPE_X1000 as i64),
        ("MACHINETYPE_SAM460", sys::MACHINETYPE_SAM460 as i64),
        ("MACHINETYPE_CHUNYE", sys::MACHINETYPE_CHUNYE as i64),
        ("MACHINETYPE_X5000_20", sys::MACHINETYPE_X5000_20 as i64),
        ("MACHINETYPE_X5000_40", sys::MACHINETYPE_X5000_40 as i64),
        ("MACHINETYPE_A1222", sys::MACHINETYPE_A1222 as i64),
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
    assert!(checks.len() >= 37, "the check list was emptied");
}

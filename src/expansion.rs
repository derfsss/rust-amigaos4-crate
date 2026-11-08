//! expansion.library's main interface, safe (feature `expansion`):
//! [`machine_info`] (GetMachineInfo).
//!
//! The rest of the main interface is Zorro autoconfiguration and DOS
//! mount plumbing (ConfigDev lists, board memory, MakeDosNode,
//! AddBootNode): system-only, left in `sys`. PCI devices are in
//! [`crate::pci`].

use core::ffi::{c_char, CStr};

use crate::error::Result;
use crate::sys::{self, ExpansionIFace, TagItem, TAG_DONE};

crate::iface::iface!(iface, ExpansionIFace, IExpansion, b"expansion.library\0", b"main\0", 50);

/// What GetMachineInfo reports.
#[derive(Clone, Copy, Debug)]
pub struct MachineInfo {
    /// MACHINETYPE_* (`sys`).
    pub machine: u32,
    /// The machine's name ("AmigaOne", "Pegasos II", ...).
    pub name: &'static str,
    /// Memory available to AmigaOS, bytes.
    pub memory: u32,
    /// All physical memory, bytes (V54.50; None on older versions).
    pub total_physical: Option<u64>,
    /// Space-separated feature words ("bus.pci", ...).
    pub extensions: &'static str,
    /// How many tags the call understood (5 on a current system).
    pub tags_understood: u32,
}

impl MachineInfo {
    /// Whether `word` is one of the extension words.
    pub fn has_extension(&self, word: &str) -> bool {
        self.extensions.split_whitespace().any(|w| w == word)
    }
}

fn s(p: *const c_char) -> &'static str {
    if p.is_null() {
        return "";
    }
    // SAFETY: GetMachineInfo's strings are expansion.library's own,
    // read-only and kept for the system's life.
    unsafe { CStr::from_ptr(p) }.to_str().unwrap_or("")
}

/// GetMachineInfo, every tag at its own width (the total physical memory
/// is a 64-bit value: a 32-bit slot for it would be overrun).
pub fn machine_info() -> Result<MachineInfo> {
    let i = iface()?;
    let mut machine = 0u32;
    let mut name: *const c_char = core::ptr::null();
    let mut memory = 0u32;
    let mut ext: *const c_char = core::ptr::null();
    let mut total = 0u64;
    let tags = [
        TagItem { ti_Tag: sys::GMIT_MACHINE, ti_Data: &mut machine as *mut u32 as u32 },
        TagItem { ti_Tag: sys::GMIT_MACHINE_STRING, ti_Data: &mut name as *mut *const c_char as u32 },
        TagItem { ti_Tag: sys::GMIT_MEMORY_AVAILABLE, ti_Data: &mut memory as *mut u32 as u32 },
        TagItem { ti_Tag: sys::GMIT_EXTENSIONS, ti_Data: &mut ext as *mut *const c_char as u32 },
        TagItem { ti_Tag: sys::GMIT_TOTAL_PHYSICAL_MEMORY, ti_Data: &mut total as *mut u64 as u32 },
        TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
    ];
    // SAFETY: an open interface; each tag points at storage of the type
    // the autodoc gives it, on this stack.
    let n = unsafe { ((*i).GetMachineInfo)(i, tags.as_ptr()) };
    Ok(MachineInfo {
        machine,
        name: s(name),
        memory,
        total_physical: (n >= 5 && total != 0).then_some(total),
        extensions: s(ext),
        tags_understood: n,
    })
}

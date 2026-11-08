//! Reading ELF files with elf.library, safe (feature `elf`, with `dos`).
//!
//! - [`ElfFile`]: OpenElf on a file (OET_Filename) or on bytes borrowed for
//!   its life (OET_MemoryAddr); CloseElf(CET_CloseAll) on drop.
//! - Everything it returns is copied: the [`ElfHeader`], the
//!   [`SectionInfo`]s with their names, a section's bytes, [`Symbol`]s by
//!   name (SymbolQuery), by address, or all of them (ScanSymbolTable).
//!   elf.library loads the sections it reads into memory: a symbol has its
//!   value in the file and its address in that loaded copy.
//!
//! - [`SharedObject`]: a shared object loaded into this program (DLOpen),
//!   its symbols' addresses by name (DLSym), unloaded on drop (DLClose).
//!
//! Not wrapped: ElfLoadSeg, RelocateSection, data segment copies, another
//! program's handle (use `dos::process::SegList` to load and run a
//! program).

use alloc::string::String;
use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::cstr::c_copy;
use crate::error::{AmigaError, Result};
use crate::sys::{self, ElfIFace, TagItem, APTR, TAG_DONE, TAG_USER};

crate::iface::iface!(iface, ElfIFace, IElf, b"elf.library\0", b"main\0", 50);

const OET_FILENAME: u32 = TAG_USER + 1;
const OET_MEMORY_ADDR: u32 = TAG_USER + 3;
const CET_CLOSE_ALL: u32 = TAG_USER + 4;
const EAT_NUM_SECTIONS: u32 = TAG_USER + 1;
const EAT_ELF_HEADER: u32 = TAG_USER + 4;
const EAT_SECTION_STRING_TABLE: u32 = TAG_USER + 5;
const GST_SECTION_INDEX: u32 = TAG_USER + 1;
const GST_LOAD: u32 = TAG_USER + 3;
const SST_LOAD: u32 = TAG_USER + 1;
const SQ_BYNAME: u32 = 0x01;
const SQ_BYVALUE: u32 = 0x02;
const SHT_NOBITS: u32 = 8;
/// struct Elf32_SymbolQuery (36 bytes, pack 2): Flags 0, Name 4,
/// NameLength 8, Value 12, Found (16-bit BOOL) 16, Sym 20.
const SQ_SIZE: usize = 36;

fn rd<T: Copy>(p: *const u8, off: usize) -> T {
    // SAFETY: callers pass a live ELF structure (or one of ours) and a
    // field offset from libraries/elf.h (tests/layout/elf.c).
    unsafe { core::ptr::read_unaligned(p.add(off) as *const T) }
}

fn string_at(p: *const u8) -> String {
    if p.is_null() {
        return String::new();
    }
    // SAFETY: callers pass a NUL-terminated string the handle owns; copied.
    String::from_utf8_lossy(unsafe { core::ffi::CStr::from_ptr(p as *const core::ffi::c_char) }.to_bytes()).into_owned()
}

/// The ELF header's fields, copied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElfHeader {
    /// e_ident: class (1 = 32-bit), data (2 = big-endian), version.
    pub class: u8,
    pub data: u8,
    /// e_type: 1 relocatable, 2 executable, 3 shared object.
    pub kind: u16,
    /// e_machine: 20 = PowerPC.
    pub machine: u16,
    pub entry: u32,
    pub section_count: u16,
    pub string_section: u16,
}

/// A section header, copied, with its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionInfo {
    pub index: u32,
    pub name: String,
    pub kind: u32,
    pub flags: u32,
    pub addr: u32,
    pub offset: u32,
    pub size: u32,
    pub link: u32,
    pub info: u32,
    pub align: u32,
    pub entry_size: u32,
}

/// A symbol, copied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    /// st_value as in the file (what readelf shows).
    pub value: u32,
    /// Where it is in elf.library's loaded copy of the file (its sections
    /// are loaded into memory): what [`ElfFile::symbol_at`] takes. 0 for a
    /// symbol in no loaded section.
    pub address: u32,
    pub size: u32,
    /// STT_*: 0 none, 1 object, 2 function, 3 section, 4 file.
    pub kind: u8,
    /// STB_*: 0 local, 1 global, 2 weak.
    pub bind: u8,
    pub section: u16,
}

impl Symbol {
    /// From a 16-byte Elf32_Sym, a name and the loaded address.
    fn from_sym(sym: *const u8, name: String, address: u32) -> Self {
        let info: u8 = rd(sym, 12);
        Self { name, value: rd(sym, 4), size: rd(sym, 8), kind: info & 0xF, bind: info >> 4, section: rd(sym, 14), address }
    }
}

/// An ELF file opened with elf.library; CloseElf on drop. From a file, or
/// from bytes borrowed for `'a`.
pub struct ElfFile<'a> {
    e: *mut ElfIFace,
    h: APTR,
    _src: PhantomData<&'a [u8]>,
}

impl ElfFile<'static> {
    /// OpenElf(OET_Filename).
    pub fn open(path: &str) -> Result<Self> {
        let name = c_copy(path.as_bytes())?;
        // SAFETY: the name lives across the call.
        unsafe { Self::open_tags(&[TagItem { ti_Tag: OET_FILENAME, ti_Data: name.as_ptr() as u32 }]) }
    }
}

impl<'a> ElfFile<'a> {
    /// OpenElf(OET_MemoryAddr): an ELF image in memory, borrowed while the
    /// handle lives.
    pub fn from_memory(image: &'a [u8]) -> Result<Self> {
        if image.len() < 52 || &image[..4] != b"\x7fELF" {
            return Err(AmigaError::InvalidArgument("not an ELF image"));
        }
        // SAFETY: the image is borrowed for the handle's life.
        unsafe { Self::open_tags(&[TagItem { ti_Tag: OET_MEMORY_ADDR, ti_Data: image.as_ptr() as u32 }]) }
    }

    /// # Safety
    /// What the tags point to lives as long as the handle.
    unsafe fn open_tags(first: &[TagItem]) -> Result<Self> {
        let e = iface()?;
        crate::dos::process()?;
        let mut tags: Vec<TagItem> = first.iter().map(|t| TagItem { ti_Tag: t.ti_Tag, ti_Data: t.ti_Data }).collect();
        tags.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        // SAFETY: an open interface in a Process; the caller's tags.
        let h = unsafe { ((*e).OpenElf)(e, tags.as_mut_ptr()) };
        if h.is_null() {
            return Err(AmigaError::InvalidArgument("OpenElf refused the file (missing, or not ELF)"));
        }
        Ok(Self { e, h, _src: PhantomData })
    }

    fn attr(&self, tag: u32) -> Option<u32> {
        let mut v: u32 = 0;
        let mut tags = [TagItem { ti_Tag: tag, ti_Data: &mut v as *mut u32 as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        // SAFETY: our handle; the tag's data is the address of a u32.
        let n = unsafe { ((*self.e).GetElfAttrs)(self.e, self.h, tags.as_mut_ptr()) };
        if n >= 1 { Some(v) } else { None }
    }

    /// The ELF header, copied.
    pub fn header(&self) -> Result<ElfHeader> {
        let p = self.attr(EAT_ELF_HEADER).filter(|&p| p != 0).ok_or(AmigaError::NullPointer)? as *const u8;
        Ok(ElfHeader {
            class: rd(p, 4),
            data: rd(p, 5),
            kind: rd(p, 16),
            machine: rd(p, 18),
            entry: rd(p, 24),
            section_count: rd(p, 48),
            string_section: rd(p, 50),
        })
    }

    /// The number of sections.
    pub fn section_count(&self) -> u32 {
        self.attr(EAT_NUM_SECTIONS).unwrap_or(0)
    }

    fn shdr(&self, index: u32) -> Option<*const u8> {
        let mut tags = [TagItem { ti_Tag: GST_SECTION_INDEX, ti_Data: index }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        // SAFETY: our handle; the header returned is the handle's.
        let p = unsafe { ((*self.e).GetSectionHeader)(self.e, self.h, tags.as_mut_ptr()) } as *const u8;
        if p.is_null() { None } else { Some(p) }
    }

    /// GetElfString: string `offset` of string-table section `section`.
    fn string(&self, section: u32, offset: u32) -> String {
        if section > 0xFFFF || offset > 0xFFFF {
            return String::new();
        }
        // SAFETY: our handle; the string is the handle's, copied.
        string_at(unsafe { ((*self.e).GetElfString)(self.e, self.h, section as u16, offset as u16) })
    }

    /// Section `index`'s header and name, copied.
    pub fn section(&self, index: u32) -> Option<SectionInfo> {
        if index >= self.section_count() {
            return None;
        }
        let p = self.shdr(index)?;
        let strtab = self.attr(EAT_SECTION_STRING_TABLE).unwrap_or(0);
        let name_off: u32 = rd(p, 0);
        Some(SectionInfo {
            index,
            name: self.string(strtab, name_off),
            kind: rd(p, 4),
            flags: rd(p, 8),
            addr: rd(p, 12),
            offset: rd(p, 16),
            size: rd(p, 20),
            link: rd(p, 24),
            info: rd(p, 28),
            align: rd(p, 32),
            entry_size: rd(p, 36),
        })
    }

    /// Every section, copied.
    pub fn sections(&self) -> Vec<SectionInfo> {
        (0..self.section_count().min(65535)).filter_map(|i| self.section(i)).collect()
    }

    /// The first section called `name`.
    pub fn section_by_name(&self, name: &str) -> Option<SectionInfo> {
        self.sections().into_iter().find(|s| s.name == name)
    }

    /// GetSection(GST_Load): section `index`'s contents, copied (empty for
    /// a section with no bytes in the file, such as .bss).
    pub fn section_data(&self, index: u32) -> Option<Vec<u8>> {
        let s = self.section(index)?;
        if s.kind == SHT_NOBITS || s.size == 0 {
            return Some(Vec::new());
        }
        let mut tags = [
            TagItem { ti_Tag: GST_SECTION_INDEX, ti_Data: index },
            TagItem { ti_Tag: GST_LOAD, ti_Data: 1 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: our handle; the section is loaded into the handle's
        // memory, sh_size bytes, and copied before anything unloads it.
        let p = unsafe { ((*self.e).GetSection)(self.e, self.h, tags.as_mut_ptr()) } as *const u8;
        if p.is_null() {
            return None;
        }
        // SAFETY: as above.
        Some(unsafe { core::slice::from_raw_parts(p, s.size as usize) }.to_vec())
    }

    fn query(&self, q: &mut [u8; SQ_SIZE]) -> bool {
        // SAFETY: our handle and one Elf32_SymbolQuery of ours, whose name
        // (or name buffer) lives across the call.
        unsafe { ((*self.e).SymbolQuery)(self.e, self.h, 1, q.as_mut_ptr() as *mut sys::Elf32_SymbolQuery) };
        rd::<u16>(q.as_ptr(), 16) != 0
    }

    /// SymbolQuery by name.
    pub fn find_symbol(&self, name: &str) -> Option<Symbol> {
        let n = c_copy(name.as_bytes()).ok()?;
        let mut q = [0u8; SQ_SIZE];
        q[0..4].copy_from_slice(&SQ_BYNAME.to_be_bytes());
        q[4..8].copy_from_slice(&(n.as_ptr() as u32).to_be_bytes());
        if !self.query(&mut q) {
            return None;
        }
        Some(self.queried(&q, name.into()))
    }

    /// A SymbolQuery result: Value is the loaded address, and the Sym copy
    /// holds st_value relative to its section, so
    /// the file's st_value is the section's address plus it.
    fn queried(&self, q: &[u8; SQ_SIZE], name: String) -> Symbol {
        let mut s = Symbol::from_sym(q[20..].as_ptr(), name, rd(q.as_ptr(), 12));
        if s.section != 0 && s.section < 0xFF00 {
            if let Some(sec) = self.section(s.section as u32) {
                s.value = s.value.wrapping_add(sec.addr);
            }
        }
        s
    }

    /// SymbolQuery by value: the symbol at `address` in the loaded copy
    /// ([`Symbol::address`]), with its name.
    pub fn symbol_at(&self, address: u32) -> Option<Symbol> {
        let mut name = [0u8; 256];
        let mut q = [0u8; SQ_SIZE];
        q[0..4].copy_from_slice(&SQ_BYVALUE.to_be_bytes());
        q[4..8].copy_from_slice(&(name.as_mut_ptr() as u32).to_be_bytes());
        q[8..12].copy_from_slice(&(name.len() as u32 - 1).to_be_bytes());
        q[12..16].copy_from_slice(&address.to_be_bytes());
        if !self.query(&mut q) {
            return None;
        }
        let end = name.iter().position(|&b| b == 0).unwrap_or(name.len() - 1);
        Some(self.queried(&q, String::from_utf8_lossy(&name[..end]).into_owned()))
    }

    /// ScanSymbolTable: every symbol, copied.
    pub fn symbols(&self) -> Vec<Symbol> {
        let mut out: Vec<Symbol> = Vec::new();
        // struct Hook: MinNode (8), h_Entry 8, h_SubEntry 12, h_Data 16.
        let mut hook = [0u32; 5];
        let entry: extern "C" fn(*mut u32, APTR, *const u8) -> u32 = scan_entry;
        hook[2] = entry as usize as u32;
        hook[4] = &mut out as *mut Vec<Symbol> as u32;
        let mut tags = [TagItem { ti_Tag: SST_LOAD, ti_Data: 1 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        // SAFETY: our handle; a hook that elf.library calls on this task
        // during the call only, its data our Vec (not touched otherwise
        // until the call returns).
        unsafe { ((*self.e).ScanSymbolTable)(self.e, self.h, hook.as_mut_ptr() as *mut sys::Hook, tags.as_mut_ptr()) };
        out
    }
}

/// ScanSymbolTable's hook: (hook, handle, struct SymbolMsg *).
extern "C" fn scan_entry(hook: *mut u32, _handle: APTR, msg: *const u8) -> u32 {
    if hook.is_null() || msg.is_null() {
        return 0;
    }
    // SAFETY: our hook (built in `symbols`) whose h_Data is the Vec being
    // filled; the message is a live struct SymbolMsg (Sym 0, AbsValue 4,
    // Name 8) for this call.
    unsafe {
        let out = &mut *(core::ptr::read(hook.add(4)) as *mut Vec<Symbol>);
        let sym: *const u8 = rd(msg, 0);
        if !sym.is_null() && out.len() < 1 << 20 {
            out.push(Symbol::from_sym(sym, string_at(rd(msg, 8)), rd(msg, 4)));
        }
    }
    1
}

impl Drop for ElfFile<'_> {
    fn drop(&mut self) {
        let mut tags = [TagItem { ti_Tag: CET_CLOSE_ALL, ti_Data: 1 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        // SAFETY: our handle (opened by name or memory, not reopened),
        // freed once.
        unsafe { ((*self.e).CloseElf)(self.e, self.h, tags.as_mut_ptr()) };
    }
}

const OET_ELF_HANDLE: u32 = TAG_USER + 4;
const CET_RE_CLOSE: u32 = TAG_USER + 5;
/// dos/dostags.h GSLI_ElfHandle; dos/dos.h GPSLF_RUN.
const GSLI_ELF_HANDLE: u32 = TAG_USER + 4000 + 4;
const GPSLF_RUN: u32 = 1 << 4;
const ELF32_NO_ERROR: usize = 0;
const ELF32_RTLD_GLOBAL: u32 = 0x0001;

/// Unload a registered shared object and close the program's re-opened
/// handle (the panic path; see `crate::track`).
unsafe fn release_shared_object(so: usize, root: usize) {
    if let Ok(e) = iface() {
        // SAFETY: the registry calls this once, for an object still loaded
        // with its root handle open.
        unsafe { ((*e).DLClose)(e, root as APTR, so as APTR) };
        SharedObject::reclose(e, root as APTR);
    }
}

/// A shared object (.so) loaded into this program (DLOpen, elf.library
/// 52.2): its symbols' addresses by name. Unloaded with everything it
/// pulled in (DLClose) on drop. Not `Send`: it is part of the calling
/// Process's image.
///
/// Opening runs the object's start-up code, which expects the C runtime
/// it was built for: SOBJS: holds newlib objects and SOBJS:clib4/ clib4
/// ones. A program without a C runtime (the crate's no_std programs)
/// most likely cannot load them: libz.so's start-up read through a NULL pointer
/// inside DLOpen on QEMU's AmigaOne. Hence
/// [`open`](Self::open) is `unsafe`.
/// [`symbol`](Self::symbol) gives an address as a number; using it as a
/// function or data pointer is the caller's `unsafe` step, and it dangles
/// once the object is dropped.
///
/// A relative name is looked for in the current directory, PROGDIR:,
/// PROGDIR:sobjs/ and SOBJS:.
pub struct SharedObject {
    _t: crate::track::Tracked,
    e: *mut ElfIFace,
    /// This program's ELF handle, re-opened (OET_ElfHandle): the DL calls
    /// need it open while they run.
    root: APTR,
    so: APTR,
    _p: PhantomData<*mut ()>,
}

impl SharedObject {
    /// DLOpen `name` with its symbols global (ELF32_RTLD_GLOBAL).
    ///
    /// # Safety
    ///
    /// The object and everything it needs (DT_NEEDED) must be built for
    /// the C runtime this program has, and its start-up code must be sound
    /// to run in this Process.
    #[track_caller]
    pub unsafe fn open(name: &str) -> Result<Self> {
        let e = iface()?;
        let dos = crate::dos::process()?;
        let n = c_copy(name.as_bytes())?;
        let mut eh: APTR = core::ptr::null_mut();
        // SAFETY: an open interface in a Process; GetProcSegList(NULL,
        // GPSLF_RUN) is the seglist of the code running now (this
        // program), and GSLI_ElfHandle writes one pointer.
        unsafe {
            let seg = ((*dos).GetProcSegList)(dos, core::ptr::null(), GPSLF_RUN);
            let tags = [TagItem { ti_Tag: GSLI_ELF_HANDLE, ti_Data: &mut eh as *mut APTR as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
            if seg != 0 {
                ((*dos).GetSegListInfo)(dos, seg, tags.as_ptr());
            }
        }
        if eh.is_null() {
            return Err(AmigaError::InvalidArgument("this program has no ELF handle (not an ELF loadable)"));
        }
        let mut tags = [TagItem { ti_Tag: OET_ELF_HANDLE, ti_Data: eh as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        // SAFETY: our own program's handle, re-opened as GSLI_ElfHandle's
        // note says, closed with CET_ReClose.
        let root = unsafe { ((*e).OpenElf)(e, tags.as_mut_ptr()) };
        if root.is_null() {
            return Err(AmigaError::InvalidArgument("OpenElf refused this program's handle"));
        }
        // SAFETY: the re-opened root handle and a NUL-terminated name.
        let so = unsafe { ((*e).DLOpen)(e, root, n.as_ptr(), ELF32_RTLD_GLOBAL) };
        if so.is_null() {
            Self::reclose(e, root);
            return Err(AmigaError::InvalidArgument("DLOpen could not load the object or one it needs"));
        }
        Ok(Self { e, root, so, _t: crate::track::register("shared object", so as usize, root as usize, release_shared_object), _p: PhantomData })
    }

    fn reclose(e: *mut ElfIFace, root: APTR) {
        let mut tags = [TagItem { ti_Tag: CET_RE_CLOSE, ti_Data: 1 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        // SAFETY: a handle opened with OET_ElfHandle, closed once with
        // CET_ReClose (CET_CloseAll would free the program's own image).
        unsafe { ((*e).CloseElf)(e, root, tags.as_mut_ptr()) };
    }

    /// DLSym: the address of `name` in this object, None if it has no such
    /// symbol. An address of 0 is a real answer.
    pub fn symbol(&self, name: &str) -> Option<usize> {
        let n = c_copy(name.as_bytes()).ok()?;
        let mut at: APTR = core::ptr::null_mut();
        // SAFETY: our open root and object handles, a NUL-terminated name
        // and one pointer to fill.
        let err = unsafe { ((*self.e).DLSym)(self.e, self.root, self.so, n.as_ptr(), &mut at) } as usize;
        (err == ELF32_NO_ERROR).then_some(at as usize)
    }
}

impl Drop for SharedObject {
    fn drop(&mut self) {
        // SAFETY: our object, unloaded once while the root handle is still
        // open; then the root closed.
        unsafe { ((*self.e).DLClose)(self.e, self.root, self.so) };
        Self::reclose(self.e, self.root);
    }
}

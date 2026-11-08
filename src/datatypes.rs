//! What a file is, and its contents, through datatypes.library, without a
//! window (feature `datatypes`, with `dos`; calls need a Process).
//!
//! - [`examine`] / [`examine_memory`]: ObtainDataTypeA for a file or bytes,
//!   the [`DataTypeInfo`] copied, ReleaseDataType. [`list_all`]: every
//!   installed datatype (the DTST_RAM walk). [`dt_string`]: GetDTString.
//! - [`DtObject`]: NewDTObjectA on a file or on borrowed bytes (pictures in
//!   PMODE_V43, not remapped to a screen), DisposeDTObject on drop. Its
//!   attributes are copied out: [`get_u32`](DtObject::get_u32),
//!   [`data_type`](DtObject::data_type),
//!   [`bitmap_header`](DtObject::bitmap_header),
//!   [`methods`](DtObject::methods), [`text`](DtObject::text); with
//!   `intuition`, [`read_pixels`](DtObject::read_pixels) reads a picture as
//!   RGB/RGBA/ARGB bytes (PDTM_READPIXELARRAY);
//!   [`save_iff`](DtObject::save_iff) writes it to a file as IFF
//!   (DTM_WRITE). With `graphics`, [`new_picture`](DtObject::new_picture)
//!   makes a picture from pixels (through a bitmap it hands over) to save.
//!
//! Not wrapped: showing an object in a window (AddDTObject and the rest),
//! printing, writing in a datatype's own format (DTWM_RAW), tools, ARexx, the
//! clipboard source, SetDTAttrs.

use alloc::string::String;
use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::cstr::c_copy;
use crate::dos::fs::{Lock, LockMode};
use crate::error::{AmigaError, Result};
use crate::sys::{self, DataTypesIFace, TagItem, APTR, TAG_DONE};

crate::iface::iface!(iface, DataTypesIFace, IDataTypes, b"datatypes.library\0", b"main\0", 45);

/// Group ids (dth_GroupID) as four-character codes.
pub mod group {
    pub const SYSTEM: [u8; 4] = *b"syst";
    pub const TEXT: [u8; 4] = *b"text";
    pub const DOCUMENT: [u8; 4] = *b"docu";
    pub const SOUND: [u8; 4] = *b"soun";
    pub const INSTRUMENT: [u8; 4] = *b"inst";
    pub const MUSIC: [u8; 4] = *b"musi";
    pub const PICTURE: [u8; 4] = *b"pict";
    pub const ANIMATION: [u8; 4] = *b"anim";
    pub const MOVIE: [u8; 4] = *b"movi";
}

/// Attribute tags (datatypes/datatypesclass.h, pictureclass.h, textclass.h).
pub mod attr {
    use crate::sys::TAG_USER;
    pub const DUMMY: u32 = TAG_USER + 0x1000;
    pub const TOTAL_VERT: u32 = DUMMY + 13;
    pub const TOTAL_HORIZ: u32 = DUMMY + 17;
    pub const BASE_NAME: u32 = DUMMY + 30;
    pub const GROUP_ID: u32 = DUMMY + 31;
    pub const SOURCE_ADDRESS: u32 = DUMMY + 39;
    pub const SOURCE_SIZE: u32 = DUMMY + 40;
    pub const SOURCE_TYPE: u32 = DUMMY + 101;
    pub const DATA_TYPE: u32 = DUMMY + 103;
    pub const NOMINAL_VERT: u32 = DUMMY + 124;
    pub const NOMINAL_HORIZ: u32 = DUMMY + 125;
    pub const PDTA_MODE_ID: u32 = DUMMY + 200;
    pub const PDTA_BITMAP_HEADER: u32 = DUMMY + 201;
    pub const PDTA_BITMAP: u32 = DUMMY + 202;
    pub const PDTA_NUM_COLORS: u32 = DUMMY + 209;
    pub const PDTA_REMAP: u32 = DUMMY + 211;
    pub const PDTA_SOURCE_MODE: u32 = DUMMY + 250;
    pub const PDTA_DEST_MODE: u32 = DUMMY + 251;
    pub const TDTA_BUFFER: u32 = DUMMY + 300;
    pub const TDTA_BUFFER_LEN: u32 = DUMMY + 301;
}

/// Methods (DTM_*, PDTM_*).
pub mod method {
    pub const PROCLAYOUT: u32 = 0x602;
    pub const WRITE: u32 = 0x650;
    pub const PDTM_WRITEPIXELARRAY: u32 = 0x660;
    pub const PDTM_READPIXELARRAY: u32 = 0x661;
}

const DTST_RAM: u32 = 1;
/// DTM_WRITE modes.
const DTWM_IFF: u32 = 0;
const DTST_FILE: u32 = 2;
const DTST_MEMORY: u32 = 5;
const PMODE_V43: u32 = 1;
/// struct DataType: dtn_Header after two 14-byte Nodes.
const DTN_HEADER: usize = 28;
/// The DTERROR_* codes start here.
pub const DTERROR_UNKNOWN_DATATYPE: i32 = 2000;
pub const DTERROR_COULDNT_OPEN: i32 = 2002;

fn rd<T: Copy>(p: *const u8, off: usize) -> T {
    // SAFETY: callers pass a live datatypes structure and a field offset
    // from its header (tests/layout/datatypes.c).
    unsafe { core::ptr::read_unaligned(p.add(off) as *const T) }
}

fn string_at(p: *const u8) -> String {
    if p.is_null() {
        return String::new();
    }
    // SAFETY: callers pass a NUL-terminated string owned by a live object;
    // it is copied.
    String::from_utf8_lossy(unsafe { core::ffi::CStr::from_ptr(p as *const core::ffi::c_char) }.to_bytes()).into_owned()
}

/// A datatype's header, copied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataTypeInfo {
    /// "ILBM", "ascii", ...
    pub name: String,
    /// The class's base name ("ilbm", "ascii": CLASSES:DataTypes/<base>.datatype).
    pub base_name: String,
    /// The file name pattern it matches.
    pub pattern: String,
    /// The group ([`group`] constants).
    pub group: [u8; 4],
    /// The datatype's id (an IFF FORM type).
    pub id: [u8; 4],
    pub priority: i32,
}

impl DataTypeInfo {
    /// Copy from a live struct DataType.
    fn from_raw(dt: *const sys::DataType) -> Self {
        let h: *const u8 = rd(dt as *const u8, DTN_HEADER);
        // struct DataTypeHeader: Name 0, BaseName 4, Pattern 8, GroupID 16,
        // ID 20, Priority 30.
        Self {
            name: string_at(rd(h, 0)),
            base_name: string_at(rd(h, 4)),
            pattern: string_at(rd(h, 8)),
            group: rd::<u32>(h, 16).to_be_bytes(),
            id: rd::<u32>(h, 20).to_be_bytes(),
            priority: rd::<u16>(h, 30) as i16 as i32,
        }
    }

    /// The group's four characters as text.
    pub fn group_str(&self) -> &str {
        core::str::from_utf8(&self.group).unwrap_or("????")
    }
}

/// Run ObtainDataTypeA, copy the result and release it.
fn obtain(kind: u32, handle: APTR, tags: &[TagItem]) -> Result<DataTypeInfo> {
    let d = iface()?;
    let dos = crate::dos::process()?;
    // SAFETY: an open interface; a handle of the kind given that lives
    // across the call and a TAG_DONE-ended list.
    let dt = unsafe { ((*d).ObtainDataTypeA)(d, kind, handle, tags.as_ptr() as *mut APTR) };
    if dt.is_null() {
        return Err(crate::dos::last_error(dos));
    }
    let info = DataTypeInfo::from_raw(dt);
    // SAFETY: ours, released once after copying.
    unsafe { ((*d).ReleaseDataType)(d, dt) };
    Ok(info)
}

/// ObtainDataTypeA(DTST_FILE): what `path` is.
pub fn examine(path: &str) -> Result<DataTypeInfo> {
    let lock = Lock::new(path, LockMode::Shared)?;
    obtain(DTST_FILE, lock.bptr() as APTR, &[TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }])
}

/// ObtainDataTypeA(DTST_MEMORY): what `bytes` are.
pub fn examine_memory(bytes: &[u8]) -> Result<DataTypeInfo> {
    let tags = [
        TagItem { ti_Tag: attr::SOURCE_ADDRESS, ti_Data: bytes.as_ptr() as u32 },
        TagItem { ti_Tag: attr::SOURCE_SIZE, ti_Data: bytes.len() as u32 },
        TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
    ];
    obtain(DTST_MEMORY, core::ptr::null_mut(), &tags)
}

/// Every installed datatype, in the system's order (the DTST_RAM walk:
/// each step names the previous one).
pub fn list_all() -> Result<Vec<DataTypeInfo>> {
    let d = iface()?;
    crate::dos::process()?;
    let mut out = Vec::new();
    let mut prev: *mut sys::DataType = core::ptr::null_mut();
    loop {
        let tags = [TagItem { ti_Tag: attr::DATA_TYPE, ti_Data: prev as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        // SAFETY: an open interface; DTST_RAM with no handle, the previous
        // datatype (still obtained) as the start.
        let dt = unsafe { ((*d).ObtainDataTypeA)(d, DTST_RAM, core::ptr::null_mut(), tags.as_ptr() as *mut APTR) };
        if !prev.is_null() {
            // SAFETY: the previous step's datatype, released once.
            unsafe { ((*d).ReleaseDataType)(d, prev) };
        }
        if dt.is_null() || out.len() > 4096 {
            if !dt.is_null() {
                // SAFETY: as above.
                unsafe { ((*d).ReleaseDataType)(d, dt) };
            }
            return Ok(out);
        }
        out.push(DataTypeInfo::from_raw(dt));
        prev = dt;
    }
}

/// GetDTString: datatypes.library's text for `id` (DTERROR_* and others).
pub fn dt_string(id: u32) -> Option<String> {
    let d = iface().ok()?;
    // SAFETY: an open interface; the string is the library's, copied.
    let p = unsafe { ((*d).GetDTString)(d, id) };
    if p.is_null() { None } else { Some(string_at(p)) }
}

/// struct BitMapHeader, copied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitMapHeader {
    pub width: u16,
    pub height: u16,
    pub left: i16,
    pub top: i16,
    pub depth: u8,
    pub masking: u8,
    pub compression: u8,
    pub transparent: u16,
    pub x_aspect: u8,
    pub y_aspect: u8,
    pub page_width: i16,
    pub page_height: i16,
}

/// Pixel layouts for [`DtObject::read_pixels`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    Rgb,
    Rgba,
    Argb,
}

impl PixelFormat {
    pub fn bytes_per_pixel(self) -> usize {
        if self == PixelFormat::Rgb { 3 } else { 4 }
    }

    #[cfg_attr(not(feature = "intuition"), allow(dead_code))]
    fn code(self) -> u32 {
        match self {
            PixelFormat::Rgb => 0,
            PixelFormat::Rgba => 1,
            PixelFormat::Argb => 2,
        }
    }
}

/// How to load an object.
#[derive(Clone, Copy, Debug, Default)]
pub struct DtOptions {
    /// Refuse anything not in this group (ERROR_OBJECT_WRONG_TYPE).
    pub group: Option<[u8; 4]>,
}

/// Dispose of a registered datatypes object (the panic path; see
/// `crate::track`).
unsafe fn release_dt_object(o: usize, _: usize) {
    if let Ok(d) = iface() {
        // SAFETY: the registry calls this once, for an object not added to
        // a window.
        unsafe { ((*d).DisposeDTObject)(d, o as *mut APTR) };
    }
}

/// A datatypes object; DisposeDTObject on drop. Loaded from a file, or
/// from bytes borrowed for `'a`.
pub struct DtObject<'a> {
    _t: crate::track::Tracked,
    d: *mut DataTypesIFace,
    o: *mut APTR,
    _src: PhantomData<&'a [u8]>,
}

impl DtObject<'static> {
    /// NewDTObjectA on a file.
    pub fn from_file(path: &str, opt: &DtOptions) -> Result<Self> {
        let name = c_copy(path.as_bytes())?;
        let mut tags = Vec::with_capacity(6);
        tags.push(TagItem { ti_Tag: attr::SOURCE_TYPE, ti_Data: DTST_FILE });
        // SAFETY: the name lives across the call (NewDTObject opens the file
        // and keeps nothing of the string).
        unsafe { DtObject::new(name.as_ptr(), tags, opt) }
    }
}

impl<'a> DtObject<'a> {
    /// NewDTObjectA on `bytes` (DTST_MEMORY), which stay borrowed while the
    /// object lives.
    pub fn from_memory(bytes: &'a [u8], opt: &DtOptions) -> Result<Self> {
        let tags = alloc::vec![
            TagItem { ti_Tag: attr::SOURCE_TYPE, ti_Data: DTST_MEMORY },
            TagItem { ti_Tag: attr::SOURCE_ADDRESS, ti_Data: bytes.as_ptr() as u32 },
            TagItem { ti_Tag: attr::SOURCE_SIZE, ti_Data: bytes.len() as u32 },
        ];
        // SAFETY: no name; the bytes are borrowed for the object's life.
        unsafe { Self::new(core::ptr::null(), tags, opt) }
    }

    /// # Safety
    /// `name` is NULL or a NUL-terminated string for the call; what the
    /// tags point to lives as long as the object.
    #[track_caller]
    unsafe fn new(name: *const u8, mut tags: Vec<TagItem>, opt: &DtOptions) -> Result<Self> {
        let d = iface()?;
        let dos = crate::dos::process()?;
        if let Some(g) = opt.group {
            tags.push(TagItem { ti_Tag: attr::GROUP_ID, ti_Data: u32::from_be_bytes(g) });
        }
        // Pictures: kept in their own depth, not remapped to a screen.
        tags.push(TagItem { ti_Tag: attr::PDTA_DEST_MODE, ti_Data: PMODE_V43 });
        tags.push(TagItem { ti_Tag: attr::PDTA_REMAP, ti_Data: 0 });
        tags.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        // SAFETY: an open interface in a Process; the caller's name and a
        // TAG_DONE-ended list whose pointers outlive the object.
        let o = unsafe { ((*d).NewDTObjectA)(d, name, tags.as_mut_ptr() as *mut APTR) };
        if o.is_null() {
            return Err(crate::dos::last_error(dos));
        }
        Ok(Self { d, o, _t: crate::track::register("datatypes object", o as usize, 0, release_dt_object), _src: PhantomData })
    }

    /// GetDTAttrsA for one tag: its 32-bit value, if the object has it.
    pub fn get_u32(&self, tag: u32) -> Option<u32> {
        let mut v: u32 = 0;
        let tags = [TagItem { ti_Tag: tag, ti_Data: &mut v as *mut u32 as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        // SAFETY: our object; the tag's data is the address of a u32
        // (every OM_GET value is 32 bits).
        let n = unsafe { ((*self.d).GetDTAttrsA)(self.d, self.o, tags.as_ptr() as *mut APTR) };
        if n == 1 { Some(v) } else { None }
    }

    /// The object's datatype (DTA_DataType), copied.
    pub fn data_type(&self) -> Option<DataTypeInfo> {
        let p = self.get_u32(attr::DATA_TYPE)? as *const sys::DataType;
        if p.is_null() { None } else { Some(DataTypeInfo::from_raw(p)) }
    }

    /// A picture's BitMapHeader (PDTA_BitMapHeader), copied.
    pub fn bitmap_header(&self) -> Option<BitMapHeader> {
        let h = self.get_u32(attr::PDTA_BITMAP_HEADER)? as *const u8;
        if h.is_null() {
            return None;
        }
        Some(BitMapHeader {
            width: rd(h, 0),
            height: rd(h, 2),
            left: rd(h, 4),
            top: rd(h, 6),
            depth: rd(h, 8),
            masking: rd(h, 9),
            compression: rd(h, 10),
            transparent: rd(h, 12),
            x_aspect: rd(h, 14),
            y_aspect: rd(h, 15),
            page_width: rd(h, 16),
            page_height: rd(h, 18),
        })
    }

    /// GetDTMethods: the methods the object supports, copied.
    pub fn methods(&self) -> Vec<u32> {
        // SAFETY: our object; the ~0-terminated array is the object's,
        // copied (bounded).
        let p = unsafe { ((*self.d).GetDTMethods)(self.d, self.o) } as *const u32;
        let mut v = Vec::new();
        if p.is_null() {
            return v;
        }
        for k in 0..1024 {
            // SAFETY: element k of the terminated array, read up to its end.
            let m = unsafe { core::ptr::read_unaligned(p.add(k)) };
            if m == u32::MAX {
                break;
            }
            v.push(m);
        }
        v
    }

    pub fn supports(&self, method: u32) -> bool {
        self.methods().contains(&method)
    }

    /// A text object's contents (TDTA_Buffer, TDTA_BufferLen), copied.
    pub fn text(&self) -> Option<Vec<u8>> {
        let p = self.get_u32(attr::TDTA_BUFFER)? as *const u8;
        let n = self.get_u32(attr::TDTA_BUFFER_LEN)? as usize;
        if p.is_null() || n > 64 << 20 {
            return None;
        }
        // SAFETY: the object's buffer of the length it reports, copied.
        Some(unsafe { core::slice::from_raw_parts(p, n) }.to_vec())
    }

    /// PDTM_READPIXELARRAY: the whole picture into `dst`, `bytes_per_row`
    /// apart, in `fmt`.
    #[cfg(feature = "intuition")]
    pub fn read_pixels(&self, fmt: PixelFormat, dst: &mut [u8], bytes_per_row: usize) -> Result<()> {
        let h = self.bitmap_header().ok_or(AmigaError::InvalidArgument("not a picture"))?;
        let (w, ht) = (h.width as usize, h.height as usize);
        if w == 0 || ht == 0 || bytes_per_row < w * fmt.bytes_per_pixel() || dst.len() < bytes_per_row * (ht - 1) + w * fmt.bytes_per_pixel() {
            return Err(AmigaError::InvalidArgument("pixel buffer too small"));
        }
        let i = crate::intuition::intuition_iface()?;
        let msg: [u32; 8] = [method::PDTM_READPIXELARRAY, dst.as_mut_ptr() as u32, fmt.code(), bytes_per_row as u32, 0, 0, w as u32, ht as u32];
        // SAFETY: our object and a struct pdtBlitPixelArray naming a buffer
        // checked above to hold the whole picture.
        let r = unsafe { ((*i).IDoMethodA)(i, self.o, msg.as_ptr() as APTR) };
        if r != 0 { Ok(()) } else { Err(AmigaError::InvalidArgument("PDTM_READPIXELARRAY failed")) }
    }

    /// DTM_WRITE in IFF form (DTWM_IFF: ILBM for a picture, tested; FTXT
    /// for text and 8SVX for a sound by the classes' convention) to a new
    /// file at `path`. A file left by a
    /// failed write is deleted.
    pub fn save_iff(&self, path: &str) -> Result<()> {
        let f = crate::dos::fs::File::open(path, crate::dos::fs::OpenMode::Create)?;
        // struct dtWrite: MethodID, dtw_GInfo, dtw_FileHandle, dtw_Mode,
        // dtw_AttrList.
        let msg: [u32; 5] = [method::WRITE, 0, f.bptr(), DTWM_IFF, 0];
        // SAFETY: our object, no window or requester, and a dtWrite naming
        // an open file that lives across the call.
        let r = unsafe { ((*self.d).DoDTMethodA)(self.d, self.o, core::ptr::null_mut(), core::ptr::null_mut(), msg.as_ptr() as APTR) };
        let dos = crate::dos::process()?;
        let err = if r == 0 { Some(crate::dos::last_error(dos)) } else { None };
        let closed = f.close();
        match (err, closed) {
            (None, Ok(())) => Ok(()),
            (e, c) => {
                let _ = crate::dos::fs::delete(path);
                Err(e.or(c.err()).unwrap_or(AmigaError::InvalidArgument("DTM_WRITE failed")))
            }
        }
    }

    /// The object for raw calls, valid while self is.
    pub fn as_ptr(&self) -> *mut APTR {
        self.o
    }
}

#[cfg(feature = "graphics")]
impl DtObject<'static> {
    /// A new 24-bit picture object (picture.datatype, DTST_RAM) of `width` x
    /// `height` pixels taken from `pixels` in `fmt`, `bytes_per_row` apart,
    /// to be saved with [`save_iff`](DtObject::save_iff) or read back.
    ///
    /// The pixels go into an ARGB bitmap of our own that the object takes
    /// over (PDTA_BitMap; the picture class frees it), with the object's
    /// BitMapHeader set to the size and depth 24. picture.datatype 54 on the
    /// QEMU AmigaOne ignores PDTM_WRITEPIXELARRAY on a new DTST_RAM object
    /// (returns 0 and reads back zeros, with any source or destination
    /// mode), and writes an empty file if the header is left unset (libtest).
    pub fn new_picture(width: u16, height: u16, fmt: PixelFormat, pixels: &[u8], bytes_per_row: usize) -> Result<Self> {
        use crate::graphics::{BitMap, BitMapOptions, PixFmt};
        let (w, h, bpp) = (width as usize, height as usize, fmt.bytes_per_pixel());
        if w == 0 || h == 0 || bytes_per_row < w * bpp || pixels.len() < bytes_per_row * (h - 1) + w * bpp {
            return Err(AmigaError::InvalidArgument("pixel buffer too small"));
        }
        let mut bm = BitMap::new(w as u32, h as u32, 32, &BitMapOptions { pixel_format: Some(PixFmt::A8R8G8B8), clear: false, user_private: true })?;
        {
            let mut l = bm.lock().ok_or(AmigaError::Device("the new bitmap could not be locked"))?;
            let dst_bpr = l.bytes_per_row() as usize;
            let dst = l.bytes_mut();
            for y in 0..h {
                let src = &pixels[y * bytes_per_row..][..w * bpp];
                let row = &mut dst[y * dst_bpr..][..w * 4];
                for (d, s) in row.chunks_exact_mut(4).zip(src.chunks_exact(bpp)) {
                    d.copy_from_slice(&match fmt {
                        PixelFormat::Argb => [s[0], s[1], s[2], s[3]],
                        PixelFormat::Rgba => [s[3], s[0], s[1], s[2]],
                        PixelFormat::Rgb => [0xFF, s[0], s[1], s[2]],
                    });
                }
            }
        }
        let tags = alloc::vec![
            TagItem { ti_Tag: attr::SOURCE_TYPE, ti_Data: DTST_RAM },
            TagItem { ti_Tag: attr::GROUP_ID, ti_Data: u32::from_be_bytes(group::PICTURE) },
        ];
        // SAFETY: no name, no pointers in the tags.
        let o = unsafe { Self::new(core::ptr::null(), tags, &DtOptions::default()) }?;
        let bmh = o.get_u32(attr::PDTA_BITMAP_HEADER).unwrap_or(0) as *mut u8;
        if bmh.is_null() {
            return Err(AmigaError::InvalidArgument("picture.datatype gave no BitMapHeader"));
        }
        // SAFETY: the object's own BitMapHeader (bmh_Width at 0, bmh_Height
        // at 2, bmh_Depth at 8, bmh_PageWidth at 16, bmh_PageHeight at 18:
        // tests/layout/datatypes.c).
        unsafe {
            core::ptr::write_unaligned(bmh as *mut u16, width);
            core::ptr::write_unaligned(bmh.add(2) as *mut u16, height);
            *bmh.add(8) = 24;
            core::ptr::write_unaligned(bmh.add(16) as *mut u16, width);
            core::ptr::write_unaligned(bmh.add(18) as *mut u16, height);
        }
        let mut set = [
            TagItem { ti_Tag: attr::PDTA_BITMAP, ti_Data: bm.as_ptr() as u32 },
            TagItem { ti_Tag: attr::NOMINAL_HORIZ, ti_Data: w as u32 },
            TagItem { ti_Tag: attr::NOMINAL_VERT, ti_Data: h as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: our object, no window, a TAG_DONE-ended list. The bitmap
        // is the object's from here: pictureclass.h, PDTA_BitMap: "freed
        // by the picture class when DisposeDTObject() is called". It is
        // never freed here: SetDTAttrs returns 0 here either way, and a
        // check that read PDTA_BitMap back and freed ours on a mismatch
        // crashed graphics.library.
        unsafe { ((*o.d).SetDTAttrsA)(o.d, o.o, core::ptr::null_mut(), core::ptr::null_mut(), set.as_mut_ptr() as *mut APTR) };
        let _ = bm.into_raw();
        Ok(o)
    }
}

impl Drop for DtObject<'_> {
    fn drop(&mut self) {
        // SAFETY: our object, never added to a window, disposed once.
        unsafe { ((*self.d).DisposeDTObject)(self.d, self.o) };
    }
}


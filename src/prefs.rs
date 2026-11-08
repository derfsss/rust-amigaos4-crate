//! application.library's PrefsObjects, safe (feature `prefs_objects`;
//! interface version 2).
//!
//! Prefs objects are reference counted: a new one has one reference,
//! ALPO_Release drops it. A dictionary or array OWNS what is put into it
//! (DictSetObjectForKey / ALPOARR_AddObj take the caller's reference). So
//! [`PrefsDict`] and [`PrefsArray`] release on drop, values are moved in,
//! and reads either copy out (strings, numbers) or borrow the container
//! ([`PrefsDict::array`]).
//!
//! [`PrefsDict::write`] and [`PrefsDict::read`] are WritePrefsA and
//! ReadPrefsA: the dictionary as UTF-8 XML in a file. The name is an
//! ordinary path: a bare name lands in the current directory, not ENV:
//! (application.library 54). Not wrapped: dates, binary blobs, doubles,
//! serialisation through callbacks.

use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::{c_char, CStr};
use core::marker::PhantomData;

use crate::cstr::c_copy;
use crate::error::{AmigaError, Result};
use crate::sys::{PrefsObjectsIFace, TagItem, APTR, TAG_DONE, TAG_USER};

crate::iface::iface!(iface, PrefsObjectsIFace, IPrefsObjects, b"application.library\0", b"prefsobjects\0", 53, 2);

// libraries/application.h
const ALPO_ALLOC: u32 = TAG_USER + 1;
const ALPO_RELEASE: u32 = TAG_USER + 2000;
const ALPO_IDENTIFY: u32 = TAG_USER + 20000;
const ALPOSTR_ALLOC_SET_STRING: u32 = TAG_USER + 30000;
const ALPOSTR_GET_STRING: u32 = TAG_USER + 40002;
const ALPONUM_ALLOC_SET_LONG: u32 = TAG_USER + 30000;
const ALPONUM_ALLOC_SET_BOOL: u32 = TAG_USER + 30001;
const ALPONUM_GET_LONG: u32 = TAG_USER + 40002;
const ALPODICT_GET_COUNT: u32 = TAG_USER + 40000;
const ALPODICT_REMOVE_OBJ_FOR_KEY: u32 = TAG_USER + 40012;
const ALPOARR_GET_COUNT: u32 = TAG_USER + 40001;
const ALPOARR_ADD_OBJ: u32 = TAG_USER + 40010;
const ALPOARR_GET_OBJ_AT_INDEX: u32 = TAG_USER + 40012;
const READPREFS_FILE_NAME: u32 = TAG_USER + 2;
const WRITEPREFS_FILE_NAME: u32 = TAG_USER + 2;
const ALPOT_STRING: u32 = 1;
const ALPOT_NUMBER: u32 = 2;
const ALPOT_ARRAY: u32 = 5;
const ALPOT_DICTIONARY: u32 = 6;

type Ctor = unsafe extern "C" fn(*mut PrefsObjectsIFace, *mut APTR, *mut u32, *mut TagItem) -> *mut APTR;

/// One PrefsXxxA call with one method tag; the object and the error
/// flags it left.
///
/// # Safety
/// `obj` is NULL (for an Alloc method) or a live object of the kind `f`
/// handles; `data` is what `tag` takes.
unsafe fn call(i: *mut PrefsObjectsIFace, f: Ctor, obj: APTR, tag: u32, data: u32) -> (APTR, u32) {
    let mut err = 0u32;
    let mut tags = [TagItem { ti_Tag: tag, ti_Data: data }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
    // SAFETY: the caller's contract; a TAG_DONE-ended list and an error
    // word to fill.
    let r = unsafe { f(i, obj as *mut APTR, &mut err, tags.as_mut_ptr()) } as APTR;
    (r, err)
}

/// Release one reference (any object kind: the base-object entry).
fn release(i: *mut PrefsObjectsIFace, obj: APTR) {
    // SAFETY: a live object we hold a reference to, released once.
    unsafe { call(i, (*i).PrefsBaseObjectA, obj, ALPO_RELEASE, 0) };
}

fn identify(i: *mut PrefsObjectsIFace, obj: APTR) -> u32 {
    let mut t = 0u32;
    // SAFETY: a live object; ALPO_Identify writes a uint32.
    unsafe { call(i, (*i).PrefsBaseObjectA, obj, ALPO_IDENTIFY, &mut t as *mut u32 as u32) };
    t
}

fn string_of(i: *mut PrefsObjectsIFace, obj: APTR) -> Option<String> {
    if identify(i, obj) != ALPOT_STRING {
        return None;
    }
    let mut p: *const u8 = core::ptr::null();
    // SAFETY: a live string object; GetString writes its STRPTR, copied at
    // once.
    unsafe { call(i, (*i).PrefsStringA, obj, ALPOSTR_GET_STRING, &mut p as *mut *const u8 as u32) };
    (!p.is_null()).then(|| {
        // SAFETY: the string object's NUL-terminated text, still alive.
        String::from_utf8_lossy(unsafe { CStr::from_ptr(p as *const c_char) }.to_bytes()).into_owned()
    })
}

fn int_of(i: *mut PrefsObjectsIFace, obj: APTR) -> Option<i32> {
    if identify(i, obj) != ALPOT_NUMBER {
        return None;
    }
    let mut v = 0i32;
    // SAFETY: a live number object; GetLong writes an int32.
    unsafe { call(i, (*i).PrefsNumberA, obj, ALPONUM_GET_LONG, &mut v as *mut i32 as u32) };
    Some(v)
}

/// A value to put into a dictionary or array (moved in).
pub enum Value {
    Str(String),
    Int(i32),
    Bool(bool),
    Array(PrefsArray),
    Dict(PrefsDict),
}

/// Make the object for `v` (one reference, to be handed to a container).
fn make(i: *mut PrefsObjectsIFace, v: Value) -> Result<APTR> {
    let obj = match v {
        Value::Str(s) => {
            let c = c_copy(s.as_bytes())?;
            // SAFETY: an Alloc method with a NUL-terminated string (copied
            // by the object).
            unsafe { call(i, (*i).PrefsStringA, core::ptr::null_mut(), ALPOSTR_ALLOC_SET_STRING, c.as_ptr() as u32) }.0
        }
        // SAFETY: Alloc methods taking an int32 / BOOL by value.
        Value::Int(n) => unsafe { call(i, (*i).PrefsNumberA, core::ptr::null_mut(), ALPONUM_ALLOC_SET_LONG, n as u32) }.0,
        // SAFETY: as above.
        Value::Bool(b) => unsafe { call(i, (*i).PrefsNumberA, core::ptr::null_mut(), ALPONUM_ALLOC_SET_BOOL, b as u32) }.0,
        Value::Array(a) => a.into_raw(),
        Value::Dict(d) => d.into_raw(),
    };
    if obj.is_null() { Err(AmigaError::AllocationFailed) } else { Ok(obj) }
}

/// A prefs dictionary (ordered keys); released on drop.
pub struct PrefsDict {
    i: *mut PrefsObjectsIFace,
    obj: APTR,
    _p: PhantomData<*const ()>,
}

impl PrefsDict {
    pub fn new() -> Result<Self> {
        let i = iface()?;
        // SAFETY: the dictionary Alloc method on NULL.
        let (obj, _) = unsafe { call(i, (*i).PrefsDictionaryA, core::ptr::null_mut(), ALPO_ALLOC, 0) };
        if obj.is_null() { Err(AmigaError::AllocationFailed) } else { Ok(Self { i, obj, _p: PhantomData }) }
    }

    fn into_raw(self) -> APTR {
        let o = self.obj;
        core::mem::forget(self);
        o
    }

    /// Put `value` under `key` (replacing and releasing an old one).
    pub fn set(&mut self, key: &str, value: Value) -> Result<()> {
        let k = c_copy(key.as_bytes())?;
        let v = make(self.i, value)?;
        // SAFETY: our dictionary, an object with one reference that the
        // dictionary takes over on success, a NUL-terminated key (copied).
        let ok = unsafe { ((*self.i).DictSetObjectForKey)(self.i, self.obj as *mut APTR, v as *mut APTR, k.as_ptr()) } != 0;
        if ok {
            Ok(())
        } else {
            release(self.i, v);
            Err(AmigaError::AllocationFailed)
        }
    }

    pub fn set_str(&mut self, key: &str, s: &str) -> Result<()> {
        self.set(key, Value::Str(String::from(s)))
    }

    pub fn set_int(&mut self, key: &str, n: i32) -> Result<()> {
        self.set(key, Value::Int(n))
    }

    pub fn set_bool(&mut self, key: &str, b: bool) -> Result<()> {
        self.set(key, Value::Bool(b))
    }

    fn get_obj(&self, key: &str) -> Option<APTR> {
        let k = c_copy(key.as_bytes()).ok()?;
        // SAFETY: our dictionary and a NUL-terminated key; the result is
        // borrowed (no reference added).
        let o = unsafe { ((*self.i).DictGetObjectForKey)(self.i, self.obj as *mut APTR, k.as_ptr()) } as APTR;
        (!o.is_null()).then_some(o)
    }

    /// The string under `key` (None if missing or not a string).
    pub fn get_str(&self, key: &str) -> Option<String> {
        self.get_obj(key).and_then(|o| string_of(self.i, o))
    }

    /// The number under `key` (None if missing or not a number).
    pub fn get_int(&self, key: &str) -> Option<i32> {
        self.get_obj(key).and_then(|o| int_of(self.i, o))
    }

    /// DictGetBoolForKey.
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        let Ok(k) = c_copy(key.as_bytes()) else { return default };
        // SAFETY: our dictionary and a NUL-terminated key.
        unsafe { ((*self.i).DictGetBoolForKey)(self.i, self.obj as *mut APTR, k.as_ptr(), default as u32) != 0 }
    }

    /// The array under `key`, borrowed from the dictionary.
    pub fn array(&self, key: &str) -> Option<ArrayRef<'_>> {
        self.get_obj(key).filter(|&o| identify(self.i, o) == ALPOT_ARRAY).map(|obj| ArrayRef { i: self.i, obj, _d: PhantomData })
    }

    /// Whether the value under `key` is a dictionary.
    pub fn has_dict(&self, key: &str) -> bool {
        self.get_obj(key).map(|o| identify(self.i, o) == ALPOT_DICTIONARY).unwrap_or(false)
    }

    /// Remove `key` (its value is released).
    pub fn remove(&mut self, key: &str) -> Result<()> {
        let k = c_copy(key.as_bytes())?;
        // SAFETY: our dictionary; the key is copied by the call.
        let (_, err) = unsafe { call(self.i, (*self.i).PrefsDictionaryA, self.obj, ALPODICT_REMOVE_OBJ_FOR_KEY, k.as_ptr() as u32) };
        if err == 0 { Ok(()) } else { Err(AmigaError::InvalidArgument("no such key")) }
    }

    pub fn len(&self) -> usize {
        let mut n = 0i32;
        // SAFETY: our dictionary; GetCount writes an int32.
        unsafe { call(self.i, (*self.i).PrefsDictionaryA, self.obj, ALPODICT_GET_COUNT, &mut n as *mut i32 as u32) };
        n.max(0) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// WritePrefsA to the file `path` (an ordinary path: a bare name is
    /// in the current directory). A non-zero result is an error.
    pub fn write(&self, name: &str) -> Result<()> {
        let n = c_copy(name.as_bytes())?;
        let mut tags = [TagItem { ti_Tag: WRITEPREFS_FILE_NAME, ti_Data: n.as_ptr() as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        // SAFETY: our dictionary and a TAG_DONE-ended list with a
        // NUL-terminated name.
        let r = unsafe { ((*self.i).WritePrefsA)(self.i, self.obj as *mut APTR, tags.as_mut_ptr()) };
        if r == 0 { Ok(()) } else { Err(AmigaError::IoError(r as i32)) }
    }

    /// ReadPrefsA from the file `path`: the dictionary is cleared first,
    /// then filled (`&mut self`: nothing borrowed from it survives).
    pub fn read(&mut self, name: &str) -> Result<()> {
        let n = c_copy(name.as_bytes())?;
        let mut tags = [TagItem { ti_Tag: READPREFS_FILE_NAME, ti_Data: n.as_ptr() as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        // SAFETY: as in write.
        let r = unsafe { ((*self.i).ReadPrefsA)(self.i, self.obj as *mut APTR, tags.as_mut_ptr()) };
        if r == 0 { Ok(()) } else { Err(AmigaError::IoError(r as i32)) }
    }
}

impl Drop for PrefsDict {
    fn drop(&mut self) {
        release(self.i, self.obj);
    }
}

/// A prefs array; released on drop.
pub struct PrefsArray {
    i: *mut PrefsObjectsIFace,
    obj: APTR,
    _p: PhantomData<*const ()>,
}

impl PrefsArray {
    pub fn new() -> Result<Self> {
        let i = iface()?;
        // SAFETY: the array Alloc method on NULL.
        let (obj, _) = unsafe { call(i, (*i).PrefsArrayA, core::ptr::null_mut(), ALPO_ALLOC, 0) };
        if obj.is_null() { Err(AmigaError::AllocationFailed) } else { Ok(Self { i, obj, _p: PhantomData }) }
    }

    fn into_raw(self) -> APTR {
        let o = self.obj;
        core::mem::forget(self);
        o
    }

    /// Append `value` (moved in).
    pub fn push(&mut self, value: Value) -> Result<()> {
        let v = make(self.i, value)?;
        // SAFETY: our array and an object whose reference it takes over.
        let (_, err) = unsafe { call(self.i, (*self.i).PrefsArrayA, self.obj, ALPOARR_ADD_OBJ, v as u32) };
        if err == 0 {
            Ok(())
        } else {
            release(self.i, v);
            Err(AmigaError::AllocationFailed)
        }
    }

    pub fn len(&self) -> usize {
        ArrayRef { i: self.i, obj: self.obj, _d: PhantomData }.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Drop for PrefsArray {
    fn drop(&mut self) {
        release(self.i, self.obj);
    }
}

/// An array borrowed from a dictionary.
pub struct ArrayRef<'d> {
    i: *mut PrefsObjectsIFace,
    obj: APTR,
    _d: PhantomData<&'d PrefsDict>,
}

impl ArrayRef<'_> {
    pub fn len(&self) -> usize {
        let mut n = 0i32;
        // SAFETY: a live array; GetCount writes an int32.
        unsafe { call(self.i, (*self.i).PrefsArrayA, self.obj, ALPOARR_GET_COUNT, &mut n as *mut i32 as u32) };
        n.max(0) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn at(&self, index: usize) -> Option<APTR> {
        // struct ALPOObjIndex { PrefsObject *obj; int32 index; }
        let mut oi: [u32; 2] = [0, index as u32];
        // SAFETY: a live array; GetObjAtIndex fills obj (borrowed).
        unsafe { call(self.i, (*self.i).PrefsArrayA, self.obj, ALPOARR_GET_OBJ_AT_INDEX, oi.as_mut_ptr() as u32) };
        (oi[0] != 0).then_some(oi[0] as usize as APTR)
    }

    pub fn get_str(&self, index: usize) -> Option<String> {
        self.at(index).and_then(|o| string_of(self.i, o))
    }

    pub fn get_int(&self, index: usize) -> Option<i32> {
        self.at(index).and_then(|o| int_of(self.i, o))
    }
}

/// The array's items as Vec (strings), for convenience.
pub fn strings(a: &ArrayRef<'_>) -> Vec<String> {
    (0..a.len()).filter_map(|k| a.get_str(k)).collect()
}

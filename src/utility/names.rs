//! Named objects and namespaces (AllocNamedObjectA and friends, V39).
//!
//! - [`Named`]: an object not yet in a namespace; dropping it frees it.
//! - [`Named::publish`] puts it into a namespace (the system's root one,
//!   or a nested one inside another object made with a namespace) and
//!   gives a [`Published`]. Dropping that takes the object out again and
//!   frees it: AttemptRemNamedObject, or when someone still holds it,
//!   RemNamedObject, waiting until the last holder lets go (see `remove`
//!   for what utility.library 54 needs for either to work).
//! - [`find`] and [`find_all`] give [`Found`] handles, each holding one use
//!   of the object (released on drop), so the object stays alive while
//!   it is held.
//!
//! Dropping a `Published` while the same task holds a `Found` of it waits
//! for ever (like locking a mutex twice); a `Found` held by another task
//! only delays the drop.
//!
//! Names are compared without case by default (the current locale's
//! rules), with case when the namespace was made with `case`.

use alloc::vec::Vec;
use core::marker::PhantomData;

use super::text::c_copy;
use super::Utility;
use crate::error::{AmigaError, Result};
use crate::port::AmigaMsgPort;
use crate::sys::{self, NamedObject, TagItem, APTR, TAG_DONE};

/// A line on the serial debug output (target builds only).
macro_rules! note {
    ($($t:tt)*) => {
        #[cfg(target_os = "amigaos")]
        crate::serial_println!($($t)*);
    };
}

/// How to make a [`Named`].
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// Bytes of user space (no_Object), long-word aligned; 0 for none.
    pub user_space: u32,
    /// Its place among equal names in a namespace (Enqueue order).
    pub priority: i8,
    /// Give the object a namespace of its own, for nested objects.
    pub namespace: bool,
    /// That namespace refuses a second object with the same name.
    pub no_dups: bool,
    /// That namespace compares names with case.
    pub case: bool,
}

/// A namespace: the system's root, or the one inside an object.
#[derive(Clone, Copy)]
pub struct Space<'a> {
    obj: *mut NamedObject,
    _o: PhantomData<&'a NamedObject>,
}

impl Space<'static> {
    /// The system-wide root namespace.
    pub fn root() -> Self {
        Self { obj: core::ptr::null_mut(), _o: PhantomData }
    }
}

/// A named object outside any namespace.
pub struct Named {
    u: Utility,
    obj: *mut NamedObject,
    has_space: bool,
    user_space: u32,
    // AllocNamedObjectA's copy of the name is not documented: keep ours
    // alive as long as the object.
    _name: Vec<u8>,
}

// SAFETY: the object is utility.library's, usable from any task; changes
// need &mut self.
unsafe impl Send for Named {}

impl Named {
    pub fn new(u: Utility, name: &str, opt: Options) -> Result<Self> {
        let n = c_copy(name.as_bytes())?;
        let flags = (if opt.no_dups { sys::NSF_NODUPS } else { 0 }) | (if opt.case { sys::NSF_CASE } else { 0 });
        let tags = [
            TagItem { ti_Tag: sys::ANO_NAMESPACE, ti_Data: opt.namespace as u32 },
            TagItem { ti_Tag: sys::ANO_USERSPACE, ti_Data: opt.user_space },
            TagItem { ti_Tag: sys::ANO_PRIORITY, ti_Data: opt.priority as i32 as u32 },
            TagItem { ti_Tag: sys::ANO_FLAGS, ti_Data: flags },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: an open interface, a NUL-terminated name kept alive with
        // the object, a TAG_DONE-ended list with only the two defined flags.
        let obj = unsafe { ((*u.i).AllocNamedObjectA)(u.i, n.as_ptr(), tags.as_ptr()) };
        if obj.is_null() {
            return Err(AmigaError::AllocationFailed);
        }
        Ok(Self { u, obj, has_space: opt.namespace, user_space: opt.user_space, _name: n })
    }

    /// The name the object carries (NamedObjectName).
    pub fn name(&self) -> &str {
        name_of(self.u, self.obj)
    }

    /// The user space, zeroed or not as exec gave it.
    pub fn user_space(&mut self) -> &mut [u8] {
        // SAFETY: no_Object is NULL or `user_space` bytes owned by the
        // object, which &mut self holds exclusively (it is in no namespace,
        // so no one else can find it).
        unsafe {
            let p = (*self.obj).no_Object as *mut u8;
            if p.is_null() { &mut [] } else { core::slice::from_raw_parts_mut(p, self.user_space as usize) }
        }
    }

    /// The namespace inside this object (made with `namespace`).
    pub fn space(&self) -> Option<Space<'_>> {
        self.has_space.then_some(Space { obj: self.obj, _o: PhantomData })
    }

    /// AddNamedObject into `space`. `Err` gives the object back: the
    /// namespace refuses duplicates and has the name already.
    pub fn publish<'s>(self, space: Space<'s>) -> core::result::Result<Published<'s>, Named> {
        // SAFETY: an open interface, a namespace that outlives 's (the
        // root, or an object borrowed for 's), our object in no namespace.
        let ok = unsafe { ((*self.u.i).AddNamedObject)(self.u.i, space.obj, self.obj) } != 0;
        if ok { Ok(Published { named: Some(self), space: space.obj, _s: PhantomData }) } else { Err(self) }
    }
}

impl Drop for Named {
    fn drop(&mut self) {
        // SAFETY: an object in no namespace, held only by us; a nested
        // namespace is empty, since what was published in it borrowed
        // this object and is gone.
        unsafe { ((*self.u.i).FreeNamedObject)(self.u.i, self.obj) };
    }
}

/// A [`Named`] in a namespace, found by others until dropped.
pub struct Published<'s> {
    named: Option<Named>,
    space: *mut NamedObject,
    _s: PhantomData<Space<'s>>,
}

// SAFETY: as Named.
unsafe impl Send for Published<'_> {}

impl Published<'_> {
    fn named(&self) -> &Named {
        self.named.as_ref().expect("Published holds its object until dropped")
    }

    pub fn name(&self) -> &str {
        self.named().name()
    }

    /// The user space's address (no_Object, NULL when none): others reach
    /// it through [`Found::object`], so access is the program's to arrange.
    pub fn object(&self) -> APTR {
        // SAFETY: our object, alive while self is.
        unsafe { (*self.named().obj).no_Object }
    }

    pub fn space(&self) -> Option<Space<'_>> {
        self.named().space()
    }
}

/// Find our own object in `space` (one more use of it), releasing the
/// same-named ones met before it. False if it is not there any more.
fn find_own(n: &Named, space: *mut NamedObject) -> bool {
    let u = n.u;
    let mut last: *mut NamedObject = core::ptr::null_mut();
    loop {
        // SAFETY: an open interface, the namespace our object went into
        // (alive: the Published borrows it), our NUL-terminated name, and
        // NULL or an object of that space we hold a use of.
        let f = unsafe { ((*u.i).FindNamedObject)(u.i, space, n._name.as_ptr(), last) };
        if !last.is_null() {
            // SAFETY: the use the previous find gave us, released once.
            unsafe { ((*u.i).ReleaseNamedObject)(u.i, last) };
        }
        if f.is_null() {
            return false;
        }
        if f == n.obj {
            return true;
        }
        last = f;
    }
}

/// Take our object out of its namespace so it can be freed. False when it
/// must not be freed (another task removed it, or no memory to wait).
///
/// utility.library 54 (libtest probes):
/// - AttemptRemNamedObject succeeds, and unlinks the object, only when it
///   has exactly two uses: the creator's (AllocNamedObject's) and the
///   caller's from a find. By the creator alone it fails.
/// - RemNamedObject replies once the other holders have let go, but the
///   object is still in its namespace then (FindNamedObject finds it):
///   freeing it at that point leaves a freed node in the namespace, and
///   the next search crashes in the kernel's name compare. By the creator
///   alone (no other holder) it never replies.
///
/// So: find our own object and attempt; when others hold it, let
/// RemNamedObject's reply say they are gone, and find and attempt again.
/// Only a successful attempt frees.
fn remove(n: &Named, space: *mut NamedObject) -> bool {
    let u = n.u;
    let mut wait: Option<(AmigaMsgPort, *mut sys::Message)> = None;
    let removed = loop {
        if !find_own(n, space) {
            note!("amigaos4: named object \"{}\" was removed by another task: left to it", n.name());
            break false;
        }
        // SAFETY: our object, in a namespace, with our creator's use and the
        // found use.
        if unsafe { ((*u.i).AttemptRemNamedObject)(u.i, n.obj) } != 0 {
            break true;
        }
        // Others hold it too.
        if wait.is_none() {
            let Ok(port) = AmigaMsgPort::new() else { break leak(n) };
            let tags = [
                TagItem { ti_Tag: sys::ASOMSG_REPLYPORT, ti_Data: port.as_ptr() as u32 },
                TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
            ];
            // SAFETY: a TAG_DONE-ended list.
            let msg = unsafe { sys::exec_alloc_sys_object(sys::ASOT_MESSAGE, tags.as_ptr()) } as *mut sys::Message;
            if msg.is_null() {
                break leak(n);
            }
            wait = Some((port, msg));
        }
        let Some((port, msg)) = wait.as_ref() else { break false };
        // SAFETY: our object, the found use (RemNamedObject takes it), and
        // a message replied to our port; we wait for the reply before the
        // message is used again or freed.
        unsafe { ((*u.i).RemNamedObject)(u.i, n.obj, *msg) };
        while port.get_raw().is_none() {
            port.wait();
        }
        // SAFETY: the message, back from its reply: ln_Name is the object,
        // or NULL when another task removed it.
        let theirs = unsafe { (*(*msg as *mut sys::Node)).ln_Name }.is_null();
        if theirs {
            note!("amigaos4: named object \"{}\" was removed by another task: left to it", n.name());
            break false;
        }
        // The holders are gone: find and attempt again.
    };
    if let Some((_port, msg)) = wait {
        // SAFETY: our message, replied (or never sent), freed once.
        unsafe { sys::exec_free_sys_object(sys::ASOT_MESSAGE, msg as APTR) };
    }
    removed
}

/// Out of memory for the wait: never free an object someone may hold.
#[cfg_attr(not(target_os = "amigaos"), allow(unused_variables))]
fn leak(n: &Named) -> bool {
    note!("amigaos4: named object \"{}\" left allocated (no memory to wait for its holders)", n.name());
    false
}

impl Drop for Published<'_> {
    fn drop(&mut self) {
        if let Some(n) = self.named.take() {
            if remove(&n, self.space) {
                drop(n);
            } else {
                core::mem::forget(n);
            }
        }
    }
}

fn name_of<'a>(u: Utility, obj: *mut NamedObject) -> &'a str {
    // SAFETY: a live object; its name lives as long as it does (the caller
    // ties 'a to the object's holder).
    unsafe {
        let p = ((*u.i).NamedObjectName)(u.i, obj);
        if p.is_null() {
            return "";
        }
        core::ffi::CStr::from_ptr(p as *const core::ffi::c_char).to_str().unwrap_or("")
    }
}

/// One use of a named object someone published (FindNamedObject);
/// ReleaseNamedObject on drop.
pub struct Found<'s> {
    u: Utility,
    obj: *mut NamedObject,
    _s: PhantomData<Space<'s>>,
}

// SAFETY: a use count may be released from any task.
unsafe impl Send for Found<'_> {}

impl Found<'_> {
    pub fn name(&self) -> &str {
        name_of(self.u, self.obj)
    }

    /// The object's user space address (no_Object), NULL when none.
    /// Reading or writing it is the program's agreement with the owner.
    pub fn object(&self) -> APTR {
        // SAFETY: an object we hold a use of.
        unsafe { (*self.obj).no_Object }
    }
}

impl Drop for Found<'_> {
    fn drop(&mut self) {
        // SAFETY: the use FindNamedObject gave us, released once.
        unsafe { ((*self.u.i).ReleaseNamedObject)(self.u.i, self.obj) };
    }
}

fn find_after<'s>(u: Utility, space: Space<'s>, name: Option<&[u8]>, last: *mut NamedObject) -> Option<Found<'s>> {
    let p = name.map_or(core::ptr::null(), |n| n.as_ptr());
    // SAFETY: an open interface, a namespace alive for 's, a
    // NUL-terminated name or NULL (all), and NULL or an object of the
    // same space we still hold.
    let obj = unsafe { ((*u.i).FindNamedObject)(u.i, space.obj, p as sys::CONST_STRPTR, last) };
    (!obj.is_null()).then_some(Found { u, obj, _s: PhantomData })
}

/// The first object called `name` in `space`.
pub fn find<'s>(u: Utility, space: Space<'s>, name: &str) -> Result<Option<Found<'s>>> {
    let n = c_copy(name.as_bytes())?;
    Ok(find_after(u, space, Some(&n), core::ptr::null_mut()))
}

/// Every object called `name` (None: every object) in `space`, in
/// namespace order.
pub fn find_all<'s>(u: Utility, space: Space<'s>, name: Option<&str>) -> Result<Vec<Found<'s>>> {
    let n = match name {
        Some(s) => Some(c_copy(s.as_bytes())?),
        None => None,
    };
    let mut v: Vec<Found<'s>> = Vec::new();
    loop {
        let last = v.last().map_or(core::ptr::null_mut(), |f| f.obj);
        match find_after(u, space, n.as_deref(), last) {
            Some(f) => v.push(f),
            None => return Ok(v),
        }
    }
}

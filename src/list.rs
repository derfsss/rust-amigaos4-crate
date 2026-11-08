//! An exec list of named nodes that owns its nodes (AllocSysObject
//! ASOT_LIST and ASOT_NODE with copied names), for building the lists OS
//! calls take, and for exec's list operations: add at either end, insert
//! by priority (Enqueue), find by name (FindName, FindIName), remove,
//! pop, iterate (GetHead/GetSucc), move another list's nodes over
//! (MoveList).
//!
//! Every change needs `&mut self`, so iterating can never meet a change;
//! the list is not locked against other tasks (an exec list never is).
//! Pass [`NameList::as_ptr`] to an OS call that only reads the list.

use alloc::string::String;
use alloc::vec::Vec;
use core::marker::PhantomData;
use core::ptr::null_mut;

use crate::error::{AmigaError, Result};
use crate::sys::{self, List, Node, TagItem, APTR, TAG_DONE};

/// An exec list owning its named nodes.
pub struct NameList {
    list: *mut List,
    len: usize,
}

// SAFETY: the list and its nodes are MEMF_SHARED objects owned by this
// value; changes need &mut self.
unsafe impl Send for NameList {}
// SAFETY: &self only reads.
unsafe impl Sync for NameList {}

/// A node of a [`NameList`], borrowed.
#[derive(Clone, Copy)]
pub struct NodeRef<'a> {
    node: *mut Node,
    _l: PhantomData<&'a NameList>,
}

impl<'a> NodeRef<'a> {
    /// The node's name ("" when it has none or is not UTF-8).
    pub fn name(&self) -> &'a str {
        // SAFETY: a node of a list borrowed for 'a; its ln_Name is null or
        // a NUL-terminated copy exec made (ASONODE_CopyName).
        unsafe {
            let p = (*self.node).ln_Name;
            if p.is_null() {
                return "";
            }
            core::ffi::CStr::from_ptr(p as *const core::ffi::c_char).to_str().unwrap_or("")
        }
    }

    pub fn pri(&self) -> i8 {
        // SAFETY: as above.
        unsafe { (*self.node).ln_Pri }
    }

    pub fn node_type(&self) -> u8 {
        // SAFETY: as above.
        unsafe { (*self.node).ln_Type }
    }
}

/// What a node held, once removed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeInfo {
    pub name: String,
    pub pri: i8,
}

impl NameList {
    /// An empty list of type `list_type` (an NT_ value; 0 is NT_UNKNOWN).
    pub fn new(list_type: u8) -> Result<Self> {
        let tags = [
            TagItem { ti_Tag: sys::ASOLIST_TYPE, ti_Data: list_type as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: a TAG_DONE-ended tag list; exec initialises the list.
        let list = unsafe { sys::exec_alloc_sys_object(sys::ASOT_LIST, tags.as_ptr()) } as *mut List;
        if list.is_null() {
            return Err(AmigaError::AllocationFailed);
        }
        Ok(Self { list, len: 0 })
    }

    fn new_node(name: &str, pri: i8) -> Result<*mut Node> {
        let mut n = Vec::with_capacity(name.len() + 1);
        n.extend_from_slice(name.as_bytes());
        n.push(0);
        let tags = [
            TagItem { ti_Tag: sys::ASONODE_NAME, ti_Data: n.as_ptr() as u32 },
            TagItem { ti_Tag: sys::ASONODE_COPYNAME, ti_Data: 1 },
            TagItem { ti_Tag: sys::ASONODE_PRI, ti_Data: pri as i32 as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: a TAG_DONE-ended tag list; the name is copied by exec.
        let node = unsafe { sys::exec_alloc_sys_object(sys::ASOT_NODE, tags.as_ptr()) } as *mut Node;
        if node.is_null() { Err(AmigaError::AllocationFailed) } else { Ok(node) }
    }

    /// Add a node at the end (AddTail).
    pub fn push_back(&mut self, name: &str, pri: i8) -> Result<()> {
        let node = Self::new_node(name, pri)?;
        // SAFETY: our list and a fresh node of ours.
        unsafe { sys::exec_add_tail(self.list, node) };
        self.len += 1;
        Ok(())
    }

    /// Add a node at the front (AddHead).
    pub fn push_front(&mut self, name: &str, pri: i8) -> Result<()> {
        let node = Self::new_node(name, pri)?;
        // SAFETY: as above.
        unsafe { sys::exec_add_head(self.list, node) };
        self.len += 1;
        Ok(())
    }

    /// Insert by priority, highest first, after nodes of the same priority
    /// (Enqueue).
    pub fn enqueue(&mut self, name: &str, pri: i8) -> Result<()> {
        let node = Self::new_node(name, pri)?;
        // SAFETY: as above.
        unsafe { sys::exec_enqueue(self.list, node) };
        self.len += 1;
        Ok(())
    }

    /// Take a removed node's contents and free it.
    ///
    /// # Safety
    /// `node` was a node of this list, already removed from it.
    unsafe fn take(&mut self, node: *mut Node) -> NodeInfo {
        let info = {
            let r = NodeRef { node, _l: PhantomData };
            NodeInfo { name: String::from(r.name()), pri: r.pri() }
        };
        // SAFETY: our removed node, freed once (the caller's contract).
        unsafe { sys::exec_free_sys_object(sys::ASOT_NODE, node as APTR) };
        self.len -= 1;
        info
    }

    /// Remove the first node (RemHead).
    pub fn pop_front(&mut self) -> Option<NodeInfo> {
        // SAFETY: our list.
        let n = unsafe { sys::exec_rem_head(self.list) };
        // SAFETY: a node of ours, just removed.
        (!n.is_null()).then(|| unsafe { self.take(n) })
    }

    /// Remove the last node (RemTail).
    pub fn pop_back(&mut self) -> Option<NodeInfo> {
        // SAFETY: our list.
        let n = unsafe { sys::exec_rem_tail(self.list) };
        // SAFETY: a node of ours, just removed.
        (!n.is_null()).then(|| unsafe { self.take(n) })
    }

    fn find_raw(&self, name: &str, ignore_case: bool) -> *mut Node {
        let mut n = Vec::with_capacity(name.len() + 1);
        n.extend_from_slice(name.as_bytes());
        n.push(0);
        // SAFETY: our list and a NUL-terminated name.
        unsafe {
            if ignore_case {
                sys::exec_find_iname(self.list, n.as_ptr())
            } else {
                sys::exec_find_name(self.list, n.as_ptr())
            }
        }
    }

    /// The first node named `name` (FindName).
    pub fn find(&self, name: &str) -> Option<NodeRef<'_>> {
        let n = self.find_raw(name, false);
        (!n.is_null()).then_some(NodeRef { node: n, _l: PhantomData })
    }

    /// As `find`, ignoring case (FindIName).
    pub fn find_ignore_case(&self, name: &str) -> Option<NodeRef<'_>> {
        let n = self.find_raw(name, true);
        (!n.is_null()).then_some(NodeRef { node: n, _l: PhantomData })
    }

    /// Remove the first node named `name` (FindName + Remove).
    pub fn remove(&mut self, name: &str) -> Option<NodeInfo> {
        let n = self.find_raw(name, false);
        if n.is_null() {
            return None;
        }
        // SAFETY: a node of our list.
        unsafe { sys::exec_remove(n) };
        // SAFETY: just removed.
        Some(unsafe { self.take(n) })
    }

    /// Move all of `other`'s nodes to the end of this list (MoveList).
    pub fn append(&mut self, other: &mut NameList) {
        // SAFETY: two lists of ours; the nodes change owner with the list.
        unsafe { sys::exec_move_list(self.list, other.list) };
        self.len += other.len;
        other.len = 0;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The nodes, first to last.
    pub fn iter(&self) -> Iter<'_> {
        // SAFETY: our list.
        let first = unsafe { sys::exec_get_head(self.list) };
        Iter { next: first, _l: PhantomData }
    }

    /// The exec list, for OS calls that read it.
    pub fn as_ptr(&self) -> *mut List {
        self.list
    }
}

#[cfg(feature = "utility")]
impl NameList {
    /// SortList (utility.library V54.2): sort the nodes with `cmp`.
    /// `Err(Unsupported)` on an older utility.library.
    pub fn sort_by<F>(&mut self, cmp: F) -> Result<()>
    where
        F: Fn(NodeRef<'_>, NodeRef<'_>) -> core::cmp::Ordering + Sync,
    {
        let u = crate::utility::Utility::get()?;
        u.require(54, 2, "SortList")?;
        let f = &cmp;
        let hook = crate::hook::Hook::new(move |a: APTR, b: APTR| -> u32 {
            let a = NodeRef { node: a as *mut Node, _l: PhantomData };
            let b = NodeRef { node: b as *mut Node, _l: PhantomData };
            // The hook returns an int32 through the u32 the Hook type uses.
            (f(a, b) as i32) as u32
        })?;
        // SAFETY: an open interface (V54.2), our list (&mut self: no one
        // else walks it during the sort), a hook alive across the call
        // whose closure only reads the two nodes it is given.
        unsafe { ((*u.i).SortList)(u.i, self.list, hook.as_ptr()) };
        Ok(())
    }
}

impl Drop for NameList {
    fn drop(&mut self) {
        while self.pop_front().is_some() {}
        // SAFETY: our empty list, freed once.
        unsafe { sys::exec_free_sys_object(sys::ASOT_LIST, self.list as APTR) };
        self.list = null_mut();
    }
}

/// Iterator over a [`NameList`].
pub struct Iter<'a> {
    next: *mut Node,
    _l: PhantomData<&'a NameList>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = NodeRef<'a>;
    fn next(&mut self) -> Option<NodeRef<'a>> {
        if self.next.is_null() {
            return None;
        }
        let n = self.next;
        // SAFETY: a node of the borrowed list; GetSucc returns null after the
        // last one.
        self.next = unsafe { sys::exec_get_succ(n) };
        Some(NodeRef { node: n, _l: PhantomData })
    }
}

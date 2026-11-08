//! An ordered map with `u32` keys on exec's AVL trees (AVL_AddNode,
//! AVL_FindNode, AVL_RemNodeByKey, AVL_FindFirstNode,
//! AVL_FindNextNodeByAddress, AVL_FindNextNodeByKey).
//!
//! Each entry is a heap block (the program's allocator) holding exec's
//! struct AVLNode, the key and the value; the compare functions read the
//! key. exec does not lock a tree: [`AvlMap`] changes need `&mut self`.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::marker::PhantomData;
use core::ptr::null_mut;

use crate::sys::{self, AVLNode, APTR};

/// struct AVLNode: 16 opaque bytes (tests/layout/exec.c). The key follows.
#[repr(C)]
struct Entry<V> {
    avl: [u32; 4],
    key: u32,
    value: V,
}

/// The key of an entry: the u32 after its AVLNode, whatever V is.
///
/// # Safety
/// `n` is the AVLNode at the start of an `Entry<V>`.
unsafe fn key_of(n: *const AVLNode) -> u32 {
    // SAFETY: the contract: the key is at offset 16 of a repr(C) Entry.
    unsafe { core::ptr::read((n as *const u8).add(16) as *const u32) }
}

extern "C" fn node_cmp(a: *mut AVLNode, b: *mut AVLNode) -> i32 {
    // SAFETY: exec passes two nodes of one of our trees.
    let (x, y) = unsafe { (key_of(a), key_of(b)) };
    x.cmp(&y) as i32
}

extern "C" fn key_cmp(n: *mut AVLNode, key: APTR) -> i32 {
    // SAFETY: exec passes a node of one of our trees and our key value.
    let x = unsafe { key_of(n) };
    x.cmp(&(key as usize as u32)) as i32
}

/// An ordered map from `u32` to `V`.
pub struct AvlMap<V> {
    root: *mut AVLNode,
    len: usize,
    _v: PhantomData<Box<Entry<V>>>,
}

// SAFETY: the map owns its entries (Box); changes need &mut self.
unsafe impl<V: Send> Send for AvlMap<V> {}
// SAFETY: &self only reads.
unsafe impl<V: Sync> Sync for AvlMap<V> {}

impl<V> Default for AvlMap<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> AvlMap<V> {
    pub const fn new() -> Self {
        Self { root: null_mut(), len: 0, _v: PhantomData }
    }

    fn find(&self, key: u32) -> *mut Entry<V> {
        // SAFETY: our tree (or null), our compare function.
        unsafe { sys::exec_avl_find_node(self.root, key as usize as APTR, key_cmp as APTR) as *mut Entry<V> }
    }

    /// Insert `value` under `key`; the old value comes back if there was one.
    pub fn insert(&mut self, key: u32, value: V) -> Option<V> {
        let e = self.find(key);
        if !e.is_null() {
            // SAFETY: an entry of ours, borrowed mutably through &mut self.
            return Some(core::mem::replace(unsafe { &mut (*e).value }, value));
        }
        let e = Box::into_raw(Box::new(Entry { avl: [0; 4], key, value }));
        // SAFETY: our tree's root and a fresh entry whose AVLNode comes first.
        let dup = unsafe { sys::exec_avl_add_node(&mut self.root, e as *mut AVLNode, node_cmp as APTR) };
        debug_assert!(dup.is_null(), "AvlMap: find and add disagree");
        self.len += 1;
        None
    }

    pub fn get(&self, key: u32) -> Option<&V> {
        let e = self.find(key);
        // SAFETY: an entry of ours, borrowed with &self.
        (!e.is_null()).then(|| unsafe { &(*e).value })
    }

    pub fn get_mut(&mut self, key: u32) -> Option<&mut V> {
        let e = self.find(key);
        // SAFETY: an entry of ours, borrowed with &mut self.
        (!e.is_null()).then(|| unsafe { &mut (*e).value })
    }

    pub fn contains_key(&self, key: u32) -> bool {
        !self.find(key).is_null()
    }

    /// Remove `key`; its value comes back.
    pub fn remove(&mut self, key: u32) -> Option<V> {
        // SAFETY: our tree and compare function.
        let e = unsafe { sys::exec_avl_rem_node_by_key(&mut self.root, key as usize as APTR, key_cmp as APTR) } as *mut Entry<V>;
        if e.is_null() {
            return None;
        }
        self.len -= 1;
        // SAFETY: an entry of ours, out of the tree: back into its Box.
        Some(unsafe { Box::from_raw(e) }.value)
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The entries in key order.
    pub fn iter(&self) -> Iter<'_, V> {
        // SAFETY: our tree (or null).
        let first = unsafe { sys::exec_avl_find_first_node(self.root) } as *mut Entry<V>;
        Iter { next: first, _m: PhantomData }
    }

    /// The entries with keys from `key` upwards (AVL_FindNextNodeByKey).
    pub fn range_from(&self, key: u32) -> Iter<'_, V> {
        // SAFETY: our tree and compare function.
        let first = unsafe { sys::exec_avl_find_next_node_by_key(self.root, key as usize as APTR, key_cmp as APTR) } as *mut Entry<V>;
        Iter { next: first, _m: PhantomData }
    }
}

impl<V> Drop for AvlMap<V> {
    fn drop(&mut self) {
        let keys: Vec<u32> = self.iter().map(|(k, _)| k).collect();
        for k in keys {
            self.remove(k);
        }
    }
}

/// Iterator over an [`AvlMap`] in key order.
pub struct Iter<'a, V> {
    next: *mut Entry<V>,
    _m: PhantomData<&'a AvlMap<V>>,
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = (u32, &'a V);
    fn next(&mut self) -> Option<(u32, &'a V)> {
        if self.next.is_null() {
            return None;
        }
        let e = self.next;
        // SAFETY: an entry of the borrowed tree; the next by address is
        // null after the last.
        unsafe {
            self.next = sys::exec_avl_find_next_node_by_address(e as *mut AVLNode) as *mut Entry<V>;
            Some(((*e).key, &(*e).value))
        }
    }
}

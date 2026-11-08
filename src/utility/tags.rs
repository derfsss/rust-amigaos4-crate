//! Tag lists, checked, and utility.library's calls on them.
//!
//! The OS walks a tag list until TAG_DONE and follows TAG_MORE (a pointer
//! to another list) and TAG_SKIP (skip the next n items). A Rust slice that
//! lacks TAG_DONE, skips past its end or carries a TAG_MORE pointer could
//! send it anywhere, so [`TagList::new`] and [`TagListMut::new`] walk the
//! slice the way NextTagItem does and refuse those. TAG_MORE chains are not
//! supported in the safe layer.
//!
//! The calls that change a list (filter, map, apply) never shorten it:
//! they turn items into TAG_IGNORE in place.

use core::marker::PhantomData;

use super::Utility;
use crate::error::{AmigaError, Result};
use crate::sys::{self, Tag, TagItem, TAG_DONE, TAG_MORE, TAG_SKIP};
#[cfg(test)]
use crate::sys::TAG_IGNORE;

/// How many items the OS reads from `items`: through its TAG_DONE.
fn check(items: &[TagItem]) -> Result<usize> {
    let mut i = 0usize;
    loop {
        let Some(t) = items.get(i) else {
            return Err(AmigaError::InvalidArgument("tag list has no TAG_DONE"));
        };
        match t.ti_Tag {
            TAG_DONE => return Ok(i + 1),
            TAG_MORE => return Err(AmigaError::InvalidArgument("TAG_MORE is not supported")),
            TAG_SKIP => i = i.checked_add(1 + t.ti_Data as usize).ok_or(AmigaError::InvalidArgument("TAG_SKIP"))?,
            _ => i += 1,
        }
    }
}

/// A tag-value array (FilterTagItems, TagInArray): needs a TAG_DONE.
fn check_tags(tags: &[Tag]) -> Result<()> {
    if tags.contains(&TAG_DONE) { Ok(()) } else { Err(AmigaError::InvalidArgument("tag array has no TAG_DONE")) }
}

/// A checked, borrowed tag list.
#[derive(Clone, Copy)]
pub struct TagList<'a> {
    items: &'a [TagItem],
    u: Utility,
}

impl<'a> TagList<'a> {
    pub fn new(u: Utility, items: &'a [TagItem]) -> Result<Self> {
        let n = check(items)?;
        Ok(Self { items: &items[..n], u })
    }

    fn ptr(&self) -> *const TagItem {
        self.items.as_ptr()
    }

    /// The item `p` points at (a pointer the OS returned into this list).
    fn item(&self, p: *const TagItem) -> Option<&'a TagItem> {
        if p.is_null() {
            return None;
        }
        let off = (p as usize).wrapping_sub(self.items.as_ptr() as usize) / core::mem::size_of::<TagItem>();
        self.items.get(off)
    }

    /// FindTagItem: the first item with `tag`.
    pub fn find(&self, tag: Tag) -> Option<&'a TagItem> {
        // SAFETY: an open interface and a checked list (TAG_DONE inside the
        // slice, no TAG_MORE); the result points into it.
        let p = unsafe { ((*self.u.i).FindTagItem)(self.u.i, tag, self.ptr()) };
        self.item(p)
    }

    /// GetTagData: the first `tag`'s data, or `default`.
    pub fn get(&self, tag: Tag, default: u32) -> u32 {
        // SAFETY: as in find.
        unsafe { ((*self.u.i).GetTagData)(self.u.i, tag, default, self.ptr()) }
    }

    /// The items NextTagItem yields (TAG_IGNORE and skipped items left out).
    pub fn iter(&self) -> Iter<'a> {
        Iter { list: *self, cur: self.ptr() as *mut TagItem, _l: PhantomData }
    }

    /// PackBoolTags: `initial` with each `map` item's bits set where the
    /// list has its tag with non-zero data and cleared where zero.
    pub fn pack_bool(&self, initial: u32, map: &TagList<'_>) -> u32 {
        // SAFETY: an open interface and two checked lists.
        unsafe { ((*self.u.i).PackBoolTags)(self.u.i, initial, self.ptr(), map.ptr()) }
    }

    /// The checked items, through TAG_DONE.
    pub fn as_slice(&self) -> &'a [TagItem] {
        self.items
    }
}

/// NextTagItem over a [`TagList`].
pub struct Iter<'a> {
    list: TagList<'a>,
    cur: *mut TagItem,
    _l: PhantomData<&'a TagItem>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a TagItem;

    fn next(&mut self) -> Option<&'a TagItem> {
        if self.cur.is_null() {
            return None;
        }
        // SAFETY: `cur` is the list start or a cursor NextTagItem left
        // inside the checked list (or NULL after TAG_DONE).
        let p = unsafe { ((*self.list.u.i).NextTagItem)(self.list.u.i, &mut self.cur) };
        if p.is_null() {
            self.cur = core::ptr::null_mut();
        }
        self.list.item(p)
    }
}

/// FilterTagItems' logic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    /// Keep only the tags in the filter.
    Keep,
    /// Drop the tags in the filter.
    Drop,
}

/// A checked tag list that utility.library may change in place.
pub struct TagListMut<'a> {
    items: &'a mut [TagItem],
    u: Utility,
}

impl<'a> TagListMut<'a> {
    pub fn new(u: Utility, items: &'a mut [TagItem]) -> Result<Self> {
        let n = check(items)?;
        Ok(Self { items: &mut items[..n], u })
    }

    pub fn as_list(&self) -> TagList<'_> {
        TagList { items: self.items, u: self.u }
    }

    /// FilterTagItems: items outside (Keep) or inside (Drop) `tags` become
    /// TAG_IGNORE. `tags` is an array of tag values ending in TAG_DONE.
    /// Returns how many items are left.
    pub fn filter(&mut self, tags: &[Tag], logic: Filter) -> Result<u32> {
        check_tags(tags)?;
        let l = match logic {
            Filter::Keep => sys::TAGFILTER_AND,
            Filter::Drop => sys::TAGFILTER_NOT,
        };
        // SAFETY: an open interface, a checked list it may change (only
        // ti_Tag to TAG_IGNORE), a TAG_DONE-ended tag array.
        Ok(unsafe { ((*self.u.i).FilterTagItems)(self.u.i, self.items.as_mut_ptr(), tags.as_ptr(), l) })
    }

    /// MapTags: each item whose tag is a `map` item's tag gets that item's
    /// data as its new tag. Tags not in `map` are kept (`keep_unknown`) or
    /// become TAG_IGNORE. A map to TAG_MORE or TAG_SKIP is refused (it
    /// would turn data into a pointer or a count).
    pub fn map(&mut self, map: &TagList<'_>, keep_unknown: bool) -> Result<()> {
        if map.iter().any(|m| m.ti_Data == TAG_MORE || m.ti_Data == TAG_SKIP) {
            return Err(AmigaError::InvalidArgument("map to TAG_MORE or TAG_SKIP"));
        }
        let t = if keep_unknown { sys::MAP_KEEP_NOT_FOUND } else { sys::MAP_REMOVE_NOT_FOUND };
        // SAFETY: an open interface, a checked list it may change, a
        // checked map whose targets keep the list free of TAG_MORE/SKIP.
        unsafe { ((*self.u.i).MapTags)(self.u.i, self.items.as_mut_ptr(), map.ptr(), t) };
        Ok(())
    }

    /// ApplyTagChanges: items whose tag is also in `changes` take its data.
    pub fn apply(&mut self, changes: &TagList<'_>) {
        // SAFETY: an open interface, a checked list whose data it may
        // change, a checked change list.
        unsafe { ((*self.u.i).ApplyTagChanges)(self.u.i, self.items.as_mut_ptr(), changes.ptr()) };
    }

    /// FilterTagChanges with this list as the change list: items whose
    /// data equals `original`'s become TAG_IGNORE; with `apply`, the ones
    /// that differ are copied into `original`.
    pub fn filter_changes(&mut self, original: &mut TagListMut<'_>, apply: bool) {
        // SAFETY: an open interface, two checked lists it may change (tags
        // to TAG_IGNORE, data copied).
        unsafe {
            ((*self.u.i).FilterTagChanges)(self.u.i, self.items.as_mut_ptr(), original.items.as_mut_ptr(), apply as u32)
        };
    }
}

impl Utility {
    /// TagInArray: whether `tag` is in `tags` (tag values, TAG_DONE-ended).
    pub fn tag_in_array(&self, tag: Tag, tags: &[Tag]) -> Result<bool> {
        check_tags(tags)?;
        // SAFETY: an open interface and a TAG_DONE-ended array.
        Ok(unsafe { ((*self.i).TagInArray)(self.i, tag, tags.as_ptr()) } != 0)
    }
}

/// CloneTagItems' copy of a list. It borrows the original, so the
/// original cannot change while the clone lives, which is what
/// RefreshTagItemClones needs. Freed with FreeTagItems.
pub struct TagClone<'a> {
    raw: *mut TagItem,
    len: usize,
    original: TagList<'a>,
}

impl<'a> TagClone<'a> {
    pub fn new(original: TagList<'a>) -> Result<Self> {
        let u = original.u;
        // SAFETY: an open interface and a checked list.
        let raw = unsafe { ((*u.i).CloneTagItems)(u.i, original.ptr()) };
        if raw.is_null() {
            return Err(AmigaError::AllocationFailed);
        }
        // The clone ends in TAG_DONE; walk it as NextTagItem does (a
        // TAG_SKIP may hide a zero tag) to find its length.
        let mut i = 0usize;
        // SAFETY: a list the OS made from a checked one: no TAG_MORE, its
        // TAG_DONE reachable.
        let len = unsafe {
            loop {
                let t = &*raw.add(i);
                match t.ti_Tag {
                    TAG_DONE => break i + 1,
                    TAG_SKIP => i += 1 + t.ti_Data as usize,
                    _ => i += 1,
                }
            }
        };
        Ok(Self { raw, len, original })
    }

    fn items_mut(&mut self) -> &mut [TagItem] {
        // SAFETY: `len` items owned by this clone, borrowed mutably.
        unsafe { core::slice::from_raw_parts_mut(self.raw, self.len) }
    }

    pub fn as_list(&self) -> TagList<'_> {
        // SAFETY: `len` items owned by this clone, read while borrowed.
        let items = unsafe { core::slice::from_raw_parts(self.raw, self.len) };
        TagList { items, u: self.original.u }
    }

    /// The clone to change in place (checked again: a change through it
    /// cannot add TAG_MORE, but a clone of a TAG_SKIP list keeps its skip).
    pub fn as_mut(&mut self) -> Result<TagListMut<'_>> {
        let u = self.original.u;
        TagListMut::new(u, self.items_mut())
    }

    /// RefreshTagItemClones: put the original's data back into the clone.
    pub fn refresh(&mut self) {
        let u = self.original.u;
        // SAFETY: an open interface, a clone of `original`, which has not
        // changed since (it is borrowed for the clone's life).
        unsafe { ((*u.i).RefreshTagItemClones)(u.i, self.raw, self.original.ptr()) };
    }
}

impl Drop for TagClone<'_> {
    fn drop(&mut self) {
        let u = self.original.u;
        // SAFETY: the list CloneTagItems gave, freed once.
        unsafe { ((*u.i).FreeTagItems)(u.i, self.raw) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(tag: Tag, data: u32) -> TagItem {
        TagItem { ti_Tag: tag, ti_Data: data }
    }

    #[test]
    fn check_walks_like_next_tag_item() {
        let u = 0x8000_0001;
        assert_eq!(check(&[t(u, 1), t(TAG_DONE, 0), t(u, 2)]), Ok(2));
        assert!(check(&[t(u, 1)]).is_err(), "no TAG_DONE");
        assert!(check(&[]).is_err());
        assert!(check(&[t(TAG_MORE, 0x1234), t(TAG_DONE, 0)]).is_err(), "TAG_MORE");
        // TAG_SKIP 1 hides the zero tag after it: the list ends later.
        assert_eq!(check(&[t(TAG_SKIP, 1), t(TAG_DONE, 0), t(u, 3), t(TAG_DONE, 0)]), Ok(4));
        assert!(check(&[t(TAG_SKIP, 5), t(TAG_DONE, 0)]).is_err(), "skips past the end");
        assert!(check(&[t(TAG_SKIP, u32::MAX), t(TAG_DONE, 0)]).is_err());
        assert_eq!(check(&[t(TAG_IGNORE, 0), t(TAG_DONE, 0)]), Ok(2));
        assert!(check_tags(&[1, 2, TAG_DONE]).is_ok());
        assert!(check_tags(&[1, 2]).is_err());
    }
}

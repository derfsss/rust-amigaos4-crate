//! Split virtqueue rings over raw memory.
//!
//! A split virtqueue is three areas: a descriptor table (16 bytes per
//! entry), the driver's avail ring and the device's used ring. This module
//! computes where they go and reads and writes them. It holds no memory of
//! its own: [`Virtqueue`](super::Virtqueue) owns the DMA memory and hands a
//! pointer here.
//!
//! Byte order: a legacy device uses the guest's own order (big-endian on
//! AmigaOS); a modern (VIRTIO_F_VERSION_1) device always uses
//! little-endian. [`Endian`] says which.
//!
//! Cache maintenance: before the device reads what the CPU wrote, the
//! descriptor and avail areas are flushed; before the CPU reads what the
//! device wrote, the used entries are invalidated. Correct on cache
//! non-coherent machines and harmless on coherent ones.

/// The descriptor continues in `next`.
pub const DESC_F_NEXT: u16 = 1;
/// The device writes this buffer (rather than reads it).
pub const DESC_F_WRITE: u16 = 2;
/// The buffer is a table of descriptors (VIRTIO_F_INDIRECT_DESC): one ring
/// descriptor carries a whole chain. See [`write_indirect`].
pub const DESC_F_INDIRECT: u16 = 4;

/// Write entry `i` of an indirect descriptor table at `table` (16 bytes an
/// entry: addr, len, flags, next), in the ring's byte order. The table's
/// `next` fields index the table, not the ring; its last entry has no
/// DESC_F_NEXT. Put the table in the ring with one descriptor:
/// `set_desc(i, table_phys, 16 * n, DESC_F_INDIRECT, 0)` (DESC_F_WRITE is
/// not allowed on it; the entries carry their own).
///
/// The table must be in DMA memory the device can read, written before the
/// ring descriptor is published. On a cache non-coherent machine the table
/// must also be flushed (the ring flushes only its own areas).
///
/// # Safety
/// `table` must be valid for `16 * (i + 1)` bytes.
pub unsafe fn write_indirect(table: *mut u8, endian: Endian, i: u16, addr: u64, len: u32, flags: u16, next: u16) {
    // SAFETY: `table` is valid for 16 * (i + 1) bytes (the contract), so
    // entry `i`'s 16 bytes are in it.
    unsafe {
        let d = table.add(16 * i as usize);
        let little = match endian {
            Endian::Little => true,
            Endian::Native => cfg!(target_endian = "little"),
        };
        let (a, l, f, n) = if little {
            (addr.to_le_bytes(), len.to_le_bytes(), flags.to_le_bytes(), next.to_le_bytes())
        } else {
            (addr.to_be_bytes(), len.to_be_bytes(), flags.to_be_bytes(), next.to_be_bytes())
        };
        core::ptr::copy_nonoverlapping(a.as_ptr(), d, 8);
        core::ptr::copy_nonoverlapping(l.as_ptr(), d.add(8), 4);
        core::ptr::copy_nonoverlapping(f.as_ptr(), d.add(12), 2);
        core::ptr::copy_nonoverlapping(n.as_ptr(), d.add(14), 2);
    }
}
/// avail ring flag: the driver does not want interrupts.
pub const AVAIL_F_NO_INTERRUPT: u16 = 1;
/// used.flags: the device does not need to be notified of new buffers
/// (it is already working on the queue, or polls it). A hint.
pub const USED_F_NO_NOTIFY: u16 = 1;
/// The legacy interface's ring alignment and page size.
pub const LEGACY_ALIGN: usize = 4096;

/// Where the three areas of a split ring of `size` entries go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RingLayout {
    /// Number of entries (a power of two).
    pub size: u16,
    /// Offset of the avail ring.
    pub avail_off: usize,
    /// Offset of the used ring.
    pub used_off: usize,
    /// Bytes to allocate (a multiple of [`LEGACY_ALIGN`]).
    pub total: usize,
}

impl RingLayout {
    /// The legacy layout, which a modern device also accepts: descriptors
    /// at 0, the avail ring right after them, the used ring at the next
    /// 4096-byte boundary, the whole rounded up to 4096. `None` unless
    /// `size` is a power of two from 1 to 32768.
    pub fn legacy(size: u16) -> Option<Self> {
        if size == 0 || !size.is_power_of_two() {
            return None;
        }
        let n = size as usize;
        let avail_off = 16 * n;
        let avail_end = avail_off + 6 + 2 * n; // flags, idx, ring[n], used_event
        let used_off = align_up(avail_end, LEGACY_ALIGN);
        let used_end = used_off + 6 + 8 * n; // flags, idx, ring[n] of {id, len}, avail_event
        Some(Self { size, avail_off, used_off, total: align_up(used_end, LEGACY_ALIGN) })
    }
}

fn align_up(x: usize, a: usize) -> usize {
    (x + a - 1) & !(a - 1)
}

/// The byte order of the ring's fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endian {
    /// The CPU's own order: the legacy interface.
    Native,
    /// Little-endian: the modern interface.
    Little,
}

#[inline]
fn barrier() {
    // A full sync on the target: it orders against a DMA device, which an
    // SMP fence does not promise.
    #[cfg(target_os = "amigaos")]
    crate::arch::sync_barrier();
    #[cfg(not(target_os = "amigaos"))]
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
}

/// Orders a load before later loads: the used index before the entry it
/// covers. `lwsync` is enough for that (it is what Linux's dma_rmb() uses on
/// PowerPC); the full `sync` in [`barrier`] is kept for ordering CPU writes
/// against the device's view before a notify.
#[inline]
fn read_barrier() {
    #[cfg(target_os = "amigaos")]
    crate::arch::lwsync_barrier();
    #[cfg(not(target_os = "amigaos"))]
    core::sync::atomic::fence(core::sync::atomic::Ordering::Acquire);
}

/// # Safety
/// `_p` is valid for `_len` bytes.
#[inline]
unsafe fn flush(_p: *const u8, _len: usize) {
    // SAFETY: the range is valid (the contract).
    #[cfg(target_os = "amigaos")]
    unsafe {
        crate::arch::cache_flush(_p, _len)
    };
}

/// # Safety
/// `_p` is valid for `_len` bytes. (dcbf writes dirty lines back before
/// invalidating them, so nothing the CPU wrote is lost.)
#[inline]
unsafe fn invalidate(_p: *const u8, _len: usize) {
    // SAFETY: the range is valid (the contract).
    #[cfg(target_os = "amigaos")]
    unsafe {
        crate::arch::cache_invalidate(_p, _len)
    };
}

/// A split ring's driver side, over memory laid out by a [`RingLayout`].
pub struct SplitRing {
    base: *mut u8,
    layout: RingLayout,
    endian: Endian,
    avail_idx: u16,
    last_used: u16,
}

impl SplitRing {
    /// A ring over `layout.total` bytes at `base`, which the caller has
    /// zeroed (the avail and used indices start at 0).
    ///
    /// # Safety
    /// `base` must point to `layout.total` bytes that stay valid and are
    /// not otherwise accessed while the ring is in use.
    pub unsafe fn new(base: *mut u8, layout: RingLayout, endian: Endian) -> Self {
        Self { base, layout, endian, avail_idx: 0, last_used: 0 }
    }

    /// The layout this ring was built with.
    #[inline]
    pub fn layout(&self) -> RingLayout {
        self.layout
    }

    /// The avail index the driver has reached (published or not).
    #[inline]
    pub fn avail_idx(&self) -> u16 {
        self.avail_idx
    }

    /// The used index the driver has taken up to.
    #[inline]
    pub fn last_used(&self) -> u16 {
        self.last_used
    }

    /// # Safety
    /// `off` and the field's size lie inside the ring's `layout.total`
    /// bytes, and the ring's memory is valid.
    unsafe fn w16(&self, off: usize, v: u16) {
        // SAFETY: the field lies inside the ring's memory (this function's
        // contract).
        unsafe {
            let v = if self.endian == Endian::Little { v.to_le() } else { v };
            core::ptr::write_volatile(self.base.add(off) as *mut u16, v);
        }
    }
    /// # Safety
    /// `off` and the field's size lie inside the ring's `layout.total`
    /// bytes, and the ring's memory is valid.
    unsafe fn w32(&self, off: usize, v: u32) {
        // SAFETY: the field lies inside the ring's memory (this function's
        // contract).
        unsafe {
            let v = if self.endian == Endian::Little { v.to_le() } else { v };
            core::ptr::write_volatile(self.base.add(off) as *mut u32, v);
        }
    }
    /// # Safety
    /// `off` and the field's size lie inside the ring's `layout.total`
    /// bytes, and the ring's memory is valid.
    unsafe fn r16(&self, off: usize) -> u16 {
        // SAFETY: the field lies inside the ring's memory (this function's
        // contract).
        unsafe {
            let v = core::ptr::read_volatile(self.base.add(off) as *const u16);
            if self.endian == Endian::Little { u16::from_le(v) } else { v }
        }
    }
    /// # Safety
    /// `off` and the field's size lie inside the ring's `layout.total`
    /// bytes, and the ring's memory is valid.
    unsafe fn r32(&self, off: usize) -> u32 {
        // SAFETY: the field lies inside the ring's memory (this function's
        // contract).
        unsafe {
            let v = core::ptr::read_volatile(self.base.add(off) as *const u32);
            if self.endian == Endian::Little { u32::from_le(v) } else { v }
        }
    }
    /// A 64-bit address as two 32-bit halves in the ring's order. The halves
    /// sit low word first in little-endian, high word first in big-endian.
    /// # Safety
    /// `off` and the field's size lie inside the ring's `layout.total`
    /// bytes, and the ring's memory is valid.
    unsafe fn w64(&self, off: usize, v: u64) {
        // SAFETY: the field lies inside the ring's memory (this function's
        // contract).
        unsafe {
            let (lo, hi) = (v as u32, (v >> 32) as u32);
            let little = match self.endian {
                Endian::Little => true,
                Endian::Native => cfg!(target_endian = "little"),
            };
            if little {
                self.w32(off, lo);
                self.w32(off + 4, hi);
            } else {
                self.w32(off, hi);
                self.w32(off + 4, lo);
            }
        }
    }

    /// Fill descriptor `i`.
    ///
    /// # Safety
    /// `i` < size, and the device must not own descriptor `i`.
    pub unsafe fn set_desc(&mut self, i: u16, addr: u64, len: u32, flags: u16, next: u16) {
        // SAFETY: `i` < size (the contract), so descriptor `i`'s 16 bytes lie in
        // the ring's memory.
        unsafe {
            let d = 16 * i as usize;
            self.w64(d, addr);
            self.w32(d + 8, len);
            self.w16(d + 12, flags);
            self.w16(d + 14, next);
        }
    }

    /// Put chain head `head` in the next avail slot. Not visible to the
    /// device until [`publish`](Self::publish).
    ///
    /// # Safety
    /// The chain starting at `head` must be filled in.
    pub unsafe fn push_avail(&mut self, head: u16) {
        // SAFETY: the ring's memory is valid (`new`'s contract) and every offset
        // here lies inside its layout.
        unsafe {
            let slot = (self.avail_idx as usize) & (self.layout.size as usize - 1);
            self.w16(self.layout.avail_off + 4 + 2 * slot, head);
            self.avail_idx = self.avail_idx.wrapping_add(1);
        }
    }

    /// Make every pushed chain visible: ring entries before the index, then
    /// both areas flushed to RAM before the device is told. Notify the
    /// device after this.
    ///
    /// # Safety
    /// The ring's memory must be valid.
    pub unsafe fn publish(&mut self) {
        // SAFETY: the ring's memory is valid (`new`'s contract) and every offset
        // here lies inside its layout.
        unsafe {
            barrier();
            self.w16(self.layout.avail_off + 2, self.avail_idx);
            flush(self.base, self.layout.used_off);
            barrier();
        }
    }

    /// Ask the device for interrupts (`true`) or not (the
    /// VRING_AVAIL_F_NO_INTERRUPT hint, which a device may ignore).
    ///
    /// # Safety
    /// The ring's memory must be valid.
    pub unsafe fn set_interrupts(&mut self, on: bool) {
        // SAFETY: the ring's memory is valid (`new`'s contract) and every offset
        // here lies inside its layout.
        unsafe {
            self.w16(self.layout.avail_off, if on { 0 } else { AVAIL_F_NO_INTERRUPT });
            flush(self.base.add(self.layout.avail_off), 2);
            barrier();
        }
    }

    /// Whether the device asks to be notified of new buffers: false while
    /// it has set VRING_USED_F_NO_NOTIFY (it is processing the queue
    /// already). Read after `publish`, so a device that clears the flag
    /// after reading the old index is still notified (its clear is ordered
    /// before its next index read).
    ///
    /// # Safety
    /// The ring's memory must be valid.
    pub unsafe fn device_wants_notify(&self) -> bool {
        // SAFETY: the ring's memory is valid (`new`'s contract) and every offset
        // here lies inside its layout.
        unsafe {
            let u = self.layout.used_off;
            barrier();
            invalidate(self.base.add(u), 2);
            self.r16(u) & USED_F_NO_NOTIFY == 0
        }
    }

    /// The next used chain, if the device has returned one: (head id,
    /// bytes written).
    ///
    /// # Safety
    /// The ring's memory must be valid.
    pub unsafe fn take_used(&mut self) -> Option<(u32, u32)> {
        // SAFETY: the ring's memory is valid (`new`'s contract) and every offset
        // here lies inside its layout.
        unsafe {
            let u = self.layout.used_off;
            invalidate(self.base.add(u), 8);
            let idx = self.r16(u + 2);
            if idx == self.last_used {
                return None;
            }
            read_barrier(); // the index before the entry it covers
            let e = u + 4 + 8 * ((self.last_used as usize) & (self.layout.size as usize - 1));
            invalidate(self.base.add(e), 8);
            let id = self.r32(e);
            let len = self.r32(e + 4);
            self.last_used = self.last_used.wrapping_add(1);
            Some((id, len))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    #[test]
    fn device_wants_notify_reads_used_flags() {
        for e in [Endian::Native, Endian::Little] {
            let l = RingLayout::legacy(8).unwrap();
            let mut mem = alloc::vec![0u8; l.total];
            // SAFETY: `mem` is l.total bytes and outlives `ring`.
            let ring = unsafe { SplitRing::new(mem.as_mut_ptr(), l, e) };
            // SAFETY: as above.
            assert!(unsafe { ring.device_wants_notify() });
            let f = match e {
                Endian::Native => USED_F_NO_NOTIFY.to_ne_bytes(),
                Endian::Little => USED_F_NO_NOTIFY.to_le_bytes(),
            };
            mem[l.used_off] = f[0];
            mem[l.used_off + 1] = f[1];
            // SAFETY: as above.
            assert!(!unsafe { ring.device_wants_notify() });
        }
    }

    #[test]
    fn legacy_layout_matches_the_spec() {
        // 8 entries: 128 bytes of descriptors, avail at 128 (6 + 16 bytes),
        // used at 4096 (6 + 64 bytes), 8192 in all.
        let l = RingLayout::legacy(8).unwrap();
        assert_eq!((l.avail_off, l.used_off, l.total), (128, 4096, 8192));
        // 256 entries: avail at 4096 (6 + 512), used at 8192, 12288 in all.
        let l = RingLayout::legacy(256).unwrap();
        assert_eq!((l.avail_off, l.used_off, l.total), (4096, 8192, 12288));
        assert!(RingLayout::legacy(0).is_none());
        assert!(RingLayout::legacy(6).is_none());
    }

    /// A ring over a Vec, and a stand-in device that reads the avail ring and
    /// writes the used ring as a real one would.
    struct Fake {
        mem: Vec<u8>,
        ring: SplitRing,
        dev_avail: u16,
        dev_used: u16,
    }

    impl Fake {
        fn new(size: u16, endian: Endian) -> Self {
            let l = RingLayout::legacy(size).unwrap();
            let mut mem = vec![0u8; l.total];
            // SAFETY: `mem` is l.total bytes, moved into the Fake with the
            // ring (a Vec's heap buffer does not move).
            let ring = unsafe { SplitRing::new(mem.as_mut_ptr(), l, endian) };
            Self { mem, ring, dev_avail: 0, dev_used: 0 }
        }
        fn rd16(&self, off: usize, e: Endian) -> u16 {
            let b = [self.mem[off], self.mem[off + 1]];
            match e {
                Endian::Little => u16::from_le_bytes(b),
                Endian::Native => u16::from_ne_bytes(b),
            }
        }
        fn wr(&mut self, off: usize, bytes: &[u8]) {
            self.mem[off..off + bytes.len()].copy_from_slice(bytes);
        }
        /// Take every published avail entry and return it used with `len`.
        fn device_serve(&mut self, len: u32, e: Endian) -> Vec<u16> {
            let l = self.ring.layout();
            let idx = self.rd16(l.avail_off + 2, e);
            let mut heads = Vec::new();
            while self.dev_avail != idx {
                let slot = (self.dev_avail as usize) % l.size as usize;
                let head = self.rd16(l.avail_off + 4 + 2 * slot, e);
                heads.push(head);
                let u = l.used_off + 4 + 8 * ((self.dev_used as usize) % l.size as usize);
                let (id, ln) = match e {
                    Endian::Little => ((head as u32).to_le_bytes(), len.to_le_bytes()),
                    Endian::Native => ((head as u32).to_ne_bytes(), len.to_ne_bytes()),
                };
                self.wr(u, &id);
                self.wr(u + 4, &ln);
                self.dev_avail = self.dev_avail.wrapping_add(1);
                self.dev_used = self.dev_used.wrapping_add(1);
            }
            let used = match e {
                Endian::Little => self.dev_used.to_le_bytes(),
                Endian::Native => self.dev_used.to_ne_bytes(),
            };
            self.wr(l.used_off + 2, &used);
            heads
        }
    }

    #[test]
    fn indirect_entries_match_ring_descriptors() {
        // The same values through set_desc and write_indirect give the same
        // 16 bytes, in both byte orders.
        for e in [Endian::Little, Endian::Native] {
            let mut f = Fake::new(8, e);
            let mut table = [0u8; 32];
            // SAFETY: descriptor 1 < 8 and a 32-byte table for entry 1.
            unsafe {
                f.ring.set_desc(1, 0x1122_3344_5566_7788, 0x99aa_bbcc, DESC_F_WRITE | DESC_F_NEXT, 5);
                write_indirect(table.as_mut_ptr(), e, 1, 0x1122_3344_5566_7788, 0x99aa_bbcc, DESC_F_WRITE | DESC_F_NEXT, 5);
            }
            assert_eq!(&table[16..32], &f.mem[16..32], "{:?}", e);
            assert_eq!(&table[..16], &[0u8; 16]);
        }
    }

    #[test]
    fn indirect_little_endian_layout() {
        let mut t = [0u8; 16];
        // SAFETY: a 16-byte table for entry 0.
        unsafe { write_indirect(t.as_mut_ptr(), Endian::Little, 0, 0x0102_0304_0506_0708, 0x0a0b_0c0d, 0x0002, 0x0003) };
        assert_eq!(t, [8, 7, 6, 5, 4, 3, 2, 1, 0x0d, 0x0c, 0x0b, 0x0a, 2, 0, 3, 0]);
    }

    fn round_trip(e: Endian) {
        let mut f = Fake::new(8, e);
        // SAFETY: the Fake's memory backs the ring; descriptors 0..3 < 8.
        unsafe {
            assert_eq!(f.ring.take_used(), None);
            for i in 0..3u16 {
                f.ring.set_desc(i, 0x1000 * (i as u64 + 1), 4096, DESC_F_WRITE, 0);
                f.ring.push_avail(i);
            }
            // Pushed but not published: the device sees nothing.
            assert!(f.device_serve(4096, e).is_empty());
            f.ring.publish();
            assert_eq!(f.device_serve(4096, e), vec![0, 1, 2]);
            assert_eq!(f.ring.take_used(), Some((0, 4096)));
            assert_eq!(f.ring.take_used(), Some((1, 4096)));
            assert_eq!(f.ring.take_used(), Some((2, 4096)));
            assert_eq!(f.ring.take_used(), None);
        }
        // Descriptor 1: address 0x2000, length 4096, write-only.
        let d = 16;
        let len = &f.mem[d + 8..d + 12];
        let want = match e {
            Endian::Little => 4096u32.to_le_bytes(),
            Endian::Native => 4096u32.to_ne_bytes(),
        };
        assert_eq!(len, &want);
        assert_eq!(f.rd16(d + 12, e), DESC_F_WRITE);
    }

    #[test]
    fn round_trip_native() {
        round_trip(Endian::Native);
    }

    #[test]
    fn round_trip_little() {
        round_trip(Endian::Little);
    }

    #[test]
    fn indices_wrap_past_u16_and_the_ring() {
        let mut f = Fake::new(4, Endian::Little);
        // SAFETY: the Fake's memory backs the ring; heads 0..4 < 4 entries.
        unsafe {
            // 70000 chains through a 4-entry ring: the avail and used indices
            // wrap at 65536 and the slots at 4.
            for n in 0..70000u32 {
                let head = (n % 4) as u16;
                f.ring.set_desc(head, 0, 64, DESC_F_WRITE, 0);
                f.ring.push_avail(head);
                f.ring.publish();
                assert_eq!(f.device_serve(64, Endian::Little), vec![head]);
                assert_eq!(f.ring.take_used(), Some((head as u32, 64)));
            }
            assert_eq!(f.ring.avail_idx(), (70000u32 % 65536) as u16);
            assert_eq!(f.ring.last_used(), (70000u32 % 65536) as u16);
        }
    }

    #[test]
    fn interrupt_hint_is_the_avail_flags_word() {
        let mut f = Fake::new(8, Endian::Little);
        let off = f.ring.layout().avail_off;
        // SAFETY: the Fake's memory backs the ring.
        unsafe { f.ring.set_interrupts(false) };
        assert_eq!(f.rd16(off, Endian::Little), AVAIL_F_NO_INTERRUPT);
        // SAFETY: as above.
        unsafe { f.ring.set_interrupts(true) };
        assert_eq!(f.rd16(off, Endian::Little), 0);
    }

    #[test]
    fn a_64_bit_address_keeps_both_halves() {
        let mut f = Fake::new(8, Endian::Little);
        // SAFETY: the Fake's memory backs the ring; descriptor 0 < 8.
        unsafe { f.ring.set_desc(0, 0x1_2345_6789, 1, 0, 0) };
        let a = u64::from_le_bytes(f.mem[0..8].try_into().unwrap());
        assert_eq!(a, 0x1_2345_6789);
    }
}

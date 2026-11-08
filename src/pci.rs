//! PCI devices through expansion.library's `pci` interface, safe (feature
//! `pci`).
//!
//! - [`Pci::get`]: the interface (the program's `IPCI` when set, otherwise
//!   one the crate opens; see `iface`).
//! - [`Query`] and [`Pci::find`] / [`Pci::devices`]: FindDevice. Each
//!   result is a [`PciDevice`], freed (FreeDevice) on drop.
//! - Reading is allowed on any device, as a scanner does: IDs, class,
//!   status, configuration space (offsets below 256, aligned), the base
//!   address ranges ([`Bar`], copied out), the capability list, the
//!   interrupt number. A driver that owns the device may change what is
//!   read at any time.
//! - Everything that changes the device needs it locked:
//!   [`PciDevice::lock`] gives a [`PciLock`] (Unlock on drop; Lock never
//!   waits, a device held by another driver is `InUse`). Through it: the
//!   command register's I/O and memory decoding, an [`IoRegion`] (an I/O
//!   BAR, bounds-checked) and, `unsafe`, configuration writes, bus
//!   mastering and an [`MmioRegion`].
//!
//! What stays `unsafe`, and why:
//! - configuration writes (they move BARs, bus numbers, decoding);
//! - bus mastering (the device may then write to any memory address it is
//!   given);
//! - a memory BAR window: the CPU cannot reach memory BARs behind every
//!   bridge (not on the AmigaOne's Articia S), and a load there faults.
//!   I/O BARs go through the library, which handles the bridge.
//!
//! Not wrapped: AllocResource/FreeResource and BAR programming (hot-plug
//! set-up), Set/GetUserData, SetCapabilities, SetEndian (I/O stays in the
//! library's default mode: a little-endian register reads as a native
//! value), the Zorro calls.

use alloc::vec::Vec;

use crate::error::{AmigaError, Result};
use crate::sys::{self, PCIDevice, PCIIFace, TagItem, TAG_DONE};

crate::iface::iface!(iface, PCIIFace, IPCI, b"expansion.library\0", b"pci\0", 50);

/// Configuration-space offsets (PCI 3.0).
pub mod cfg {
    pub const VENDOR_ID: u32 = 0x00;
    pub const DEVICE_ID: u32 = 0x02;
    pub const COMMAND: u32 = 0x04;
    pub const STATUS: u32 = 0x06;
    pub const REVISION: u32 = 0x08;
    pub const HEADER_TYPE: u32 = 0x0E;
    pub const SUBSYSTEM_VENDOR_ID: u32 = 0x2C;
    pub const SUBSYSTEM_ID: u32 = 0x2E;
    pub const CAPABILITY_LIST: u32 = 0x34;
    pub const INTERRUPT_LINE: u32 = 0x3C;
    pub const INTERRUPT_PIN: u32 = 0x3D;
    /// COMMAND bits.
    pub const COMMAND_IO: u16 = 1 << 0;
    pub const COMMAND_MEMORY: u16 = 1 << 1;
    pub const COMMAND_MASTER: u16 = 1 << 2;
    /// STATUS: the device has a capability list.
    pub const STATUS_CAP_LIST: u16 = 1 << 4;
}

/// The `pci` interface of expansion.library.
#[derive(Clone, Copy)]
pub struct Pci {
    i: *mut PCIIFace,
}

// SAFETY: the interface may be called from any task.
unsafe impl Send for Pci {}
// SAFETY: as above.
unsafe impl Sync for Pci {}

/// What to look for (FindDevice's tags). Empty: every device.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Query {
    vendor: Option<u16>,
    device: Option<u16>,
    class: Option<(u32, u32)>,
    address: Option<(u8, u8, u8)>,
}

impl Query {
    pub fn any() -> Self {
        Self::default()
    }

    /// The device at bus, device, function (an exclusive search mode:
    /// combined with any other condition, the query finds nothing).
    pub fn at(bus: u8, device: u8, function: u8) -> Self {
        Self { address: Some((bus, device, function)), ..Self::default() }
    }

    pub fn vendor(mut self, id: u16) -> Self {
        self.vendor = Some(id);
        self
    }

    pub fn device(mut self, id: u16) -> Self {
        self.device = Some(id);
        self
    }

    /// Class code 0x00BBSSII (base class, subclass, programming
    /// interface) compared under `mask` (0x00FF0000: base class only,
    /// 0x00FFFF00: base class and subclass).
    pub fn class(mut self, class: u32, mask: u32) -> Self {
        self.class = Some((class & 0x00FF_FFFF, mask & 0x00FF_FFFF));
        self
    }

    fn tags(&self, index: u32) -> Result<Vec<TagItem>> {
        let mut t = Vec::with_capacity(6);
        let tag = |tag, data| TagItem { ti_Tag: tag, ti_Data: data };
        if let Some((b, d, f)) = self.address {
            if self.vendor.is_some() || self.device.is_some() || self.class.is_some() {
                return Err(AmigaError::InvalidArgument("a PCI address query takes no other condition"));
            }
            t.push(tag(sys::FDT_BUS_NR, b as u32));
            t.push(tag(sys::FDT_DEVICE_NR, d as u32));
            t.push(tag(sys::FDT_FUNCTION_NR, f as u32));
        } else {
            if let Some(v) = self.vendor {
                t.push(tag(sys::FDT_VENDOR_ID, v as u32));
            }
            if let Some(d) = self.device {
                t.push(tag(sys::FDT_DEVICE_ID, d as u32));
            }
            if let Some((c, m)) = self.class {
                t.push(tag(sys::FDT_CLASS, c));
                t.push(tag(sys::FDT_CLASS_MASK, m));
            }
            t.push(tag(sys::FDT_INDEX, index));
        }
        t.push(tag(TAG_DONE, 0));
        Ok(t)
    }
}

impl Pci {
    /// The interface; `Err(NullPointer)` if expansion.library has none.
    pub fn get() -> Result<Self> {
        iface().map(|i| Self { i })
    }

    /// The `index`th device (from 0) matching `q`, or None.
    pub fn find(&self, q: &Query, index: u32) -> Result<Option<PciDevice>> {
        let tags = q.tags(index)?;
        // SAFETY: an open interface and a TAG_DONE-ended list of FDT tags
        // in the combinations the autodoc allows (Query::tags).
        let dev = unsafe { ((*self.i).FindDevice)(self.i, tags.as_ptr()) };
        Ok((!dev.is_null()).then_some(PciDevice { pci: *self, dev }))
    }

    /// Every device matching `q`, in the library's order.
    pub fn devices(&self, q: &Query) -> Result<Vec<PciDevice>> {
        let mut v = Vec::new();
        if q.address.is_some() {
            v.extend(self.find(q, 0)?);
            return Ok(v);
        }
        while let Some(d) = self.find(q, v.len() as u32)? {
            v.push(d);
        }
        Ok(v)
    }
}

/// Where a base address range is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarKind {
    Io,
    Memory { prefetchable: bool },
}

/// One base address range, copied out of GetResourceRange.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bar {
    /// BAR number, 0..=5.
    pub index: u8,
    pub kind: BarKind,
    /// The CPU's address for a memory range (BaseAddress).
    pub base: u32,
    /// The I/O port for an I/O range, the bus address for a memory one
    /// (Physical).
    pub physical: u32,
    pub size: u32,
    /// The library's PCI_RANGE_* flags.
    pub flags: u32,
}

/// One entry of the capability list (copied).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capability {
    /// PCI_CAPABILITYID_* (9: vendor-specific, 5: MSI, 0x11: MSI-X, ...).
    pub id: u32,
    /// Its offset in configuration space.
    pub offset: u32,
}

/// How a device is locked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockMode {
    /// With other shared holders (a multi-function chip's parts).
    Shared,
    /// Alone: what a driver of a whole device takes.
    Exclusive,
}

/// A PCI device, as FindDevice returned it; FreeDevice on drop.
pub struct PciDevice {
    pci: Pci,
    dev: *mut PCIDevice,
}

impl PciDevice {
    fn r8(&self, off: u32) -> u8 {
        // SAFETY: a device we hold a reference to; offsets below 256.
        unsafe { ((*self.dev).ReadConfigByte)(self.dev, off) }
    }

    fn r16(&self, off: u32) -> u16 {
        // SAFETY: as r8; 2-aligned offsets.
        unsafe { ((*self.dev).ReadConfigWord)(self.dev, off) }
    }

    fn r32(&self, off: u32) -> u32 {
        // SAFETY: as r8; 4-aligned offsets.
        unsafe { ((*self.dev).ReadConfigLong)(self.dev, off) }
    }

    fn check(off: u32, width: u32) -> Result<()> {
        if off + width > 256 || off % width != 0 {
            return Err(AmigaError::InvalidArgument("PCI configuration offset out of range or unaligned"));
        }
        Ok(())
    }

    pub fn vendor_id(&self) -> u16 {
        self.r16(cfg::VENDOR_ID)
    }

    pub fn device_id(&self) -> u16 {
        self.r16(cfg::DEVICE_ID)
    }

    pub fn revision(&self) -> u8 {
        self.r8(cfg::REVISION)
    }

    /// Class code 0x00BBSSII.
    pub fn class_code(&self) -> u32 {
        self.r32(cfg::REVISION) >> 8
    }

    pub fn header_type(&self) -> u8 {
        self.r8(cfg::HEADER_TYPE)
    }

    /// Subsystem vendor and ID (header type 0 only).
    pub fn subsystem(&self) -> (u16, u16) {
        (self.r16(cfg::SUBSYSTEM_VENDOR_ID), self.r16(cfg::SUBSYSTEM_ID))
    }

    pub fn command(&self) -> u16 {
        self.r16(cfg::COMMAND)
    }

    pub fn status(&self) -> u16 {
        self.r16(cfg::STATUS)
    }

    pub fn interrupt_pin(&self) -> u8 {
        self.r8(cfg::INTERRUPT_PIN)
    }

    pub fn interrupt_line(&self) -> u8 {
        self.r8(cfg::INTERRUPT_LINE)
    }

    /// ReadConfigByte at `off` (below 256).
    pub fn read_config8(&self, off: u32) -> Result<u8> {
        Self::check(off, 1).map(|_| self.r8(off))
    }

    /// ReadConfigWord at `off` (2-aligned, below 256), as a native value.
    pub fn read_config16(&self, off: u32) -> Result<u16> {
        Self::check(off, 2).map(|_| self.r16(off))
    }

    /// ReadConfigLong at `off` (4-aligned, below 256), as a native value.
    pub fn read_config32(&self, off: u32) -> Result<u32> {
        Self::check(off, 4).map(|_| self.r32(off))
    }

    /// GetAddress: bus, device, function.
    pub fn address(&self) -> (u8, u8, u8) {
        let (mut b, mut d, mut f) = (0u8, 0u8, 0u8);
        // SAFETY: a device we hold; three bytes to fill.
        unsafe { ((*self.dev).GetAddress)(self.dev, &mut b, &mut d, &mut f) };
        (b, d, f)
    }

    /// GetCapabilities: the library's PCI_CAP_* flags for the device
    /// (bus master possible, bridge, not yet configured...), not the
    /// capability list.
    pub fn flags(&self) -> u32 {
        // SAFETY: a device we hold.
        unsafe { ((*self.dev).GetCapabilities)(self.dev) }
    }

    /// Base address range `index` (0..=5), or None when the BAR is unused.
    pub fn bar(&self, index: u8) -> Option<Bar> {
        if index as u32 >= sys::PCI_NUM_RESOURCE_RANGES {
            return None;
        }
        // SAFETY: a device we hold and a BAR number below 6; the range is
        // read (unaligned: the struct is packed) and freed at once.
        unsafe {
            let r = ((*self.dev).GetResourceRange)(self.dev, index);
            if r.is_null() {
                return None;
            }
            let base = core::ptr::addr_of!((*r).BaseAddress).read_unaligned();
            let size = core::ptr::addr_of!((*r).Size).read_unaligned();
            let physical = core::ptr::addr_of!((*r).Physical).read_unaligned();
            let flags = core::ptr::addr_of!((*r).Flags).read_unaligned();
            ((*self.dev).FreeResourceRange)(self.dev, r);
            let kind = if flags & sys::PCI_RANGE_IO != 0 {
                BarKind::Io
            } else {
                BarKind::Memory { prefetchable: flags & sys::PCI_RANGE_PREFETCH != 0 }
            };
            Some(Bar { index, kind, base, physical, size, flags })
        }
    }

    /// Every BAR in use.
    pub fn bars(&self) -> Vec<Bar> {
        (0..sys::PCI_NUM_RESOURCE_RANGES as u8).filter_map(|i| self.bar(i)).collect()
    }

    /// Whether this device interface has the vtable slot at `offset`
    /// (the capability calls came with a later expansion.library).
    fn has_slot(&self, offset: usize) -> bool {
        // SAFETY: a device interface we hold: its InterfaceData's
        // positive_size is the vtable's size from its start.
        let size = unsafe { (*self.dev).data.positive_size } as usize;
        offset + 4 <= size
    }

    /// The capability list as expansion.library read it (empty when the
    /// device has none, or the library has no capability calls).
    pub fn capabilities(&self) -> Vec<Capability> {
        let mut v = Vec::new();
        if !self.has_slot(core::mem::offset_of!(PCIDevice, GetNextCapability)) {
            return v;
        }
        // SAFETY: a device we hold; the list is the library's, valid while
        // the device is, walked with its own calls and copied out.
        unsafe {
            let mut c = ((*self.dev).GetFirstCapability)(self.dev);
            while !c.is_null() && v.len() < 64 {
                v.push(Capability { id: (*c).Type, offset: (*c).CapOffset });
                c = ((*self.dev).GetNextCapability)(self.dev, c);
            }
        }
        v
    }

    /// FindCapability: the first capability `id`.
    pub fn find_capability(&self, id: u32) -> Option<Capability> {
        if !self.has_slot(core::mem::offset_of!(PCIDevice, FindCapability)) {
            return None;
        }
        // SAFETY: as in capabilities.
        unsafe {
            let c = ((*self.dev).FindCapability)(self.dev, id);
            (!c.is_null()).then(|| Capability { id: (*c).Type, offset: (*c).CapOffset })
        }
    }

    /// MapInterrupt: the exec interrupt number for `IntServer`, or None
    /// when the device has no interrupt pin. INTx lines are shared and
    /// level-triggered: a server must check its own device first.
    pub fn interrupt(&self) -> Option<u32> {
        if self.interrupt_pin() == 0 {
            return None;
        }
        // SAFETY: a device we hold; MapInterrupt only reads.
        Some(unsafe { ((*self.dev).MapInterrupt)(self.dev) })
    }

    /// Lock the device (never waits): `Err(InUse)` while another driver
    /// holds it in a conflicting mode.
    pub fn lock(&mut self, mode: LockMode) -> Result<PciLock<'_>> {
        let m = match mode {
            LockMode::Shared => sys::PCI_LOCK_SHARED,
            LockMode::Exclusive => sys::PCI_LOCK_EXCLUSIVE,
        };
        // SAFETY: a device we hold; the lock is released by PciLock's drop.
        if unsafe { ((*self.dev).Lock)(self.dev, m) } == 0 {
            return Err(AmigaError::InUse);
        }
        Ok(PciLock { dev: self })
    }

    /// The raw device for code that still takes one (the `virtio`
    /// transports). Valid while self is.
    pub fn as_ptr(&self) -> *mut PCIDevice {
        self.dev
    }
}

impl Drop for PciDevice {
    fn drop(&mut self) {
        // SAFETY: the reference FindDevice gave, freed once; any lock was
        // released first (a PciLock borrows the device).
        unsafe { ((*self.pci.i).FreeDevice)(self.pci.i, self.dev) };
    }
}

/// A locked [`PciDevice`]; Unlock on drop.
pub struct PciLock<'d> {
    dev: &'d mut PciDevice,
}

impl PciLock<'_> {
    pub fn device(&self) -> &PciDevice {
        self.dev
    }

    fn set_command(&self, set: u16, clear: u16) -> u16 {
        let d = self.dev.dev;
        // SAFETY: a device we have locked; only the given COMMAND bits
        // change.
        unsafe {
            let old = ((*d).ReadConfigWord)(d, cfg::COMMAND);
            let new = (old | set) & !clear;
            if new != old {
                ((*d).WriteConfigWord)(d, cfg::COMMAND, new);
            }
            ((*d).ReadConfigWord)(d, cfg::COMMAND)
        }
    }

    /// Turn the device's I/O and memory decoding on or off; returns the
    /// command register after.
    pub fn set_decoding(&self, io: bool, memory: bool) -> u16 {
        let set = (if io { cfg::COMMAND_IO } else { 0 }) | (if memory { cfg::COMMAND_MEMORY } else { 0 });
        let clear = (cfg::COMMAND_IO | cfg::COMMAND_MEMORY) & !set;
        self.set_command(set, clear)
    }

    /// Let the device master the bus (DMA); returns the command register.
    ///
    /// # Safety
    /// From now until [`PciLock::disable_bus_master`], every address the
    /// device is given to read or write must be memory the program owns
    /// for that use (DMA bypasses Rust's rules), set up through exec's
    /// StartDMA (`mem::DmaMapping`).
    pub unsafe fn enable_bus_master(&self) -> u16 {
        self.set_command(cfg::COMMAND_MASTER, 0)
    }

    /// Stop the device mastering the bus; returns the command register.
    pub fn disable_bus_master(&self) -> u16 {
        self.set_command(0, cfg::COMMAND_MASTER)
    }

    /// WriteConfigByte.
    ///
    /// # Safety
    /// `off` is below 256, and the write leaves the bus working: no BAR,
    /// bridge bus number or decoding change that another user relies on.
    pub unsafe fn write_config8(&self, off: u32, v: u8) {
        let d = self.dev.dev;
        // SAFETY: a locked device; the rest is the caller's contract.
        unsafe { ((*d).WriteConfigByte)(d, off, v) };
    }

    /// WriteConfigWord (a native value).
    ///
    /// # Safety
    /// As [`PciLock::write_config8`]; `off` 2-aligned.
    pub unsafe fn write_config16(&self, off: u32, v: u16) {
        let d = self.dev.dev;
        // SAFETY: as above.
        unsafe { ((*d).WriteConfigWord)(d, off, v) };
    }

    /// WriteConfigLong (a native value).
    ///
    /// # Safety
    /// As [`PciLock::write_config8`]; `off` 4-aligned.
    pub unsafe fn write_config32(&self, off: u32, v: u32) {
        let d = self.dev.dev;
        // SAFETY: as above.
        unsafe { ((*d).WriteConfigLong)(d, off, v) };
    }

    /// I/O BAR `index` as a bounds-checked window.
    pub fn io(&self, index: u8) -> Result<IoRegion<'_>> {
        match self.dev.bar(index) {
            Some(b) if b.kind == BarKind::Io && b.size > 0 => Ok(IoRegion { dev: self.dev.dev, port: b.physical, size: b.size, _l: core::marker::PhantomData }),
            _ => Err(AmigaError::InvalidArgument("not an I/O BAR")),
        }
    }

    /// Memory BAR `index` as a bounds-checked window.
    ///
    /// # Safety
    /// The CPU must be able to reach this BAR at its BaseAddress (it cannot
    /// behind every bridge: not on the AmigaOne's Articia S, where a load
    /// faults); the caller has checked, for example by writing and reading
    /// back a register the device defines.
    pub unsafe fn mmio(&self, index: u8) -> Result<MmioRegion<'_>> {
        match self.dev.bar(index) {
            Some(b) if matches!(b.kind, BarKind::Memory { .. }) && b.size > 0 => {
                Ok(MmioRegion { base: b.base, size: b.size, _l: core::marker::PhantomData })
            }
            _ => Err(AmigaError::InvalidArgument("not a memory BAR")),
        }
    }
}

impl Drop for PciLock<'_> {
    fn drop(&mut self) {
        // SAFETY: the lock this guard holds, released once.
        unsafe { ((*self.dev.dev).Unlock)(self.dev.dev) };
    }
}

/// An I/O BAR of a locked device: reads and writes at offsets inside it,
/// through the library's In/Out calls (little-endian registers come back
/// as native values).
pub struct IoRegion<'l> {
    dev: *mut PCIDevice,
    port: u32,
    size: u32,
    _l: core::marker::PhantomData<&'l PciLock<'l>>,
}

impl IoRegion<'_> {
    pub fn size(&self) -> u32 {
        self.size
    }

    /// The first port (an absolute I/O address).
    pub fn port(&self) -> u32 {
        self.port
    }

    fn at(&self, off: u32, width: u32) -> Result<u32> {
        if off as u64 + width as u64 > self.size as u64 || off % width != 0 {
            return Err(AmigaError::InvalidArgument("I/O offset outside the BAR or unaligned"));
        }
        Ok(self.port + off)
    }

    pub fn read8(&self, off: u32) -> Result<u8> {
        let p = self.at(off, 1)?;
        // SAFETY: a locked device's own I/O range.
        Ok(unsafe { ((*self.dev).InByte)(self.dev, p) })
    }

    pub fn read16(&self, off: u32) -> Result<u16> {
        let p = self.at(off, 2)?;
        // SAFETY: as read8.
        Ok(unsafe { ((*self.dev).InWord)(self.dev, p) })
    }

    pub fn read32(&self, off: u32) -> Result<u32> {
        let p = self.at(off, 4)?;
        // SAFETY: as read8.
        Ok(unsafe { ((*self.dev).InLong)(self.dev, p) })
    }

    pub fn write8(&self, off: u32, v: u8) -> Result<()> {
        let p = self.at(off, 1)?;
        // SAFETY: a locked device's own I/O range (what the device does
        // with the write is its driver's business; DMA needs bus
        // mastering, which is unsafe to enable).
        unsafe { ((*self.dev).OutByte)(self.dev, p, v) };
        Ok(())
    }

    pub fn write16(&self, off: u32, v: u16) -> Result<()> {
        let p = self.at(off, 2)?;
        // SAFETY: as write8.
        unsafe { ((*self.dev).OutWord)(self.dev, p, v) };
        Ok(())
    }

    pub fn write32(&self, off: u32, v: u32) -> Result<()> {
        let p = self.at(off, 4)?;
        // SAFETY: as write8.
        unsafe { ((*self.dev).OutLong)(self.dev, p, v) };
        Ok(())
    }

    /// InByteBlock from one register (a FIFO) into `buf`.
    pub fn read_fifo8(&self, off: u32, buf: &mut [u8]) -> Result<()> {
        let p = self.at(off, 1)?;
        // SAFETY: one port of our range read buf.len() times into buf
        // (PortInc 0, BlockInc 1 unit).
        unsafe { ((*self.dev).InByteBlock)(self.dev, p, buf.as_mut_ptr(), 0, 1, buf.len() as u32) };
        Ok(())
    }

    /// InWordBlock from one register into `buf`.
    pub fn read_fifo16(&self, off: u32, buf: &mut [u16]) -> Result<()> {
        let p = self.at(off, 2)?;
        // SAFETY: as read_fifo8, u16 units.
        unsafe { ((*self.dev).InWordBlock)(self.dev, p, buf.as_mut_ptr(), 0, 1, buf.len() as u32) };
        Ok(())
    }

    /// InLongBlock from one register into `buf`.
    pub fn read_fifo32(&self, off: u32, buf: &mut [u32]) -> Result<()> {
        let p = self.at(off, 4)?;
        // SAFETY: as read_fifo8, u32 units.
        unsafe { ((*self.dev).InLongBlock)(self.dev, p, buf.as_mut_ptr(), 0, 1, buf.len() as u32) };
        Ok(())
    }

    /// OutByteBlock from `buf` to one register.
    pub fn write_fifo8(&self, off: u32, buf: &[u8]) -> Result<()> {
        let p = self.at(off, 1)?;
        // SAFETY: buf read buf.len() times (the call only reads it; the
        // SDK types the block non-const), one port of our range.
        unsafe { ((*self.dev).OutByteBlock)(self.dev, p, buf.as_ptr() as *mut u8, 0, 1, buf.len() as u32) };
        Ok(())
    }

    /// OutLongBlock from `buf` to one register.
    pub fn write_fifo32(&self, off: u32, buf: &[u32]) -> Result<()> {
        let p = self.at(off, 4)?;
        // SAFETY: as write_fifo8, u32 units.
        unsafe { ((*self.dev).OutLongBlock)(self.dev, p, buf.as_ptr() as *mut u32, 0, 1, buf.len() as u32) };
        Ok(())
    }
}

/// A memory BAR of a locked device the CPU can reach (see
/// [`PciLock::mmio`]): little-endian registers, read and written as
/// native values (byte-reversed loads and stores).
#[cfg_attr(not(target_os = "amigaos"), allow(dead_code))]
pub struct MmioRegion<'l> {
    base: u32,
    size: u32,
    _l: core::marker::PhantomData<&'l PciLock<'l>>,
}

#[cfg(target_os = "amigaos")]
impl MmioRegion<'_> {
    pub fn size(&self) -> u32 {
        self.size
    }

    fn at(&self, off: u32, width: u32) -> Result<u32> {
        if off as u64 + width as u64 > self.size as u64 || off % width != 0 {
            return Err(AmigaError::InvalidArgument("MMIO offset outside the BAR or unaligned"));
        }
        Ok(self.base + off)
    }

    pub fn read8(&self, off: u32) -> Result<u8> {
        let a = self.at(off, 1)?;
        // SAFETY: inside a memory BAR the CPU reaches (PciLock::mmio's
        // contract), aligned.
        Ok(unsafe { crate::arch::mmio_r8(a) })
    }

    pub fn read16(&self, off: u32) -> Result<u16> {
        let a = self.at(off, 2)?;
        // SAFETY: as read8.
        Ok(unsafe { crate::arch::mmio_r16(a) })
    }

    pub fn read32(&self, off: u32) -> Result<u32> {
        let a = self.at(off, 4)?;
        // SAFETY: as read8.
        Ok(unsafe { crate::arch::mmio_r32(a) })
    }

    pub fn write8(&self, off: u32, v: u8) -> Result<()> {
        let a = self.at(off, 1)?;
        // SAFETY: as read8.
        unsafe { crate::arch::mmio_w8(a, v) };
        Ok(())
    }

    pub fn write16(&self, off: u32, v: u16) -> Result<()> {
        let a = self.at(off, 2)?;
        // SAFETY: as read8.
        unsafe { crate::arch::mmio_w16(a, v) };
        Ok(())
    }

    pub fn write32(&self, off: u32, v: u32) -> Result<()> {
        let a = self.at(off, 4)?;
        // SAFETY: as read8.
        unsafe { crate::arch::mmio_w32(a, v) };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_tags() {
        let t = Query::any().vendor(0x1af4).class(0x0001_0000, 0x00ff_0000).tags(2).unwrap();
        let tags: Vec<(u32, u32)> = t.iter().map(|i| (i.ti_Tag, i.ti_Data)).collect();
        assert_eq!(
            tags,
            [(sys::FDT_VENDOR_ID, 0x1af4), (sys::FDT_CLASS, 0x10000), (sys::FDT_CLASS_MASK, 0xff0000), (sys::FDT_INDEX, 2), (TAG_DONE, 0)]
        );
        let a = Query::at(0, 3, 1).tags(0).unwrap();
        assert_eq!(a.len(), 4);
        assert!(Query::at(0, 3, 1).vendor(1).tags(0).is_err());
    }

    #[test]
    fn config_offsets() {
        assert!(PciDevice::check(0xFC, 4).is_ok());
        assert!(PciDevice::check(0x100, 1).is_err());
        assert!(PciDevice::check(0xFE, 4).is_err());
        assert!(PciDevice::check(0x02, 4).is_err());
        assert!(PciDevice::check(0x3D, 1).is_ok());
    }
}

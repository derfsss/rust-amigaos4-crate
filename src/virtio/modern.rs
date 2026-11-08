//! The modern (virtio 1.x) PCI interface, and the choice between it and
//! the legacy one.
//!
//! A modern device describes where its registers are with vendor-specific
//! PCI capabilities (virtio 1.x section 4.1.4): the common configuration,
//! the notify area, the ISR byte and the device configuration, each a BAR
//! and an offset. Every register is little-endian and accessed at its own
//! width.
//!
//! On AmigaOS a memory BAR's CPU address is the PCIResourceRange's
//! `BaseAddress` (on the Pegasos II's MV64361 `Physical` is the PCI-side
//! address, and reads through it return 0). Whether the CPU can reach the
//! BAR at all depends on the bridge: on the AmigaOne's Articia S it cannot,
//! so [`ModernPci::probe`] writes the status register and reads it back
//! before the interface is used, and [`PciTransport::open`] falls back to
//! the legacy interface when it fails.

use super::queue::Virtqueue;
use super::transport::{pci, LegacyPci, Transport};
use super::{enable_pci, STATUS_ACKNOWLEDGE};
use crate::error::{AmigaError, Result};
use crate::sys::{self, PCIDevice};

// virtio 1.x section 4.1.4: vendor-specific capability, cfg_type values.
const PCI_CAP_ID_VENDOR: u8 = 0x09;
const CAP_COMMON: u8 = 1;
const CAP_NOTIFY: u8 = 2;
const CAP_ISR: u8 = 3;
const CAP_DEVICE: u8 = 4;

// struct virtio_pci_common_cfg (virtio 1.x section 4.1.4.3).
const C_DFSELECT: u32 = 0x00;
const C_DF: u32 = 0x04;
const C_GFSELECT: u32 = 0x08;
const C_GF: u32 = 0x0C;
const C_STATUS: u32 = 0x14;
const C_Q_SELECT: u32 = 0x16;
const C_Q_SIZE: u32 = 0x18;
const C_Q_ENABLE: u32 = 0x1C;
const C_Q_NOFF: u32 = 0x1E;
const C_Q_DESC: u32 = 0x20;
const C_Q_DRIVER: u32 = 0x28;
const C_Q_DEVICE: u32 = 0x30;

const PCI_STATUS: u32 = 0x06;
const PCI_STATUS_CAP_LIST: u16 = 1 << 4;
const PCI_CAP_PTR: u32 = 0x34;
const PCI_BAR0: u32 = 0x10;

/// Queues whose notify address is kept.
const MAX_QUEUES: usize = 8;

// Register access at its own width. Safety for each: `a` is a register of a
// live device's mapped BAR (crate::arch's MMIO contract).
#[inline]
unsafe fn r8(a: u32) -> u8 {
    // SAFETY: the caller's contract above.
    unsafe { crate::arch::mmio_r8(a) }
}
#[inline]
unsafe fn w8(a: u32, v: u8) {
    // SAFETY: the caller's contract above.
    unsafe { crate::arch::mmio_w8(a, v) }
}
#[inline]
unsafe fn r16(a: u32) -> u16 {
    // SAFETY: the caller's contract above.
    unsafe { crate::arch::mmio_r16(a) }
}
#[inline]
unsafe fn w16(a: u32, v: u16) {
    // SAFETY: the caller's contract above.
    unsafe { crate::arch::mmio_w16(a, v) }
}
#[inline]
unsafe fn r32(a: u32) -> u32 {
    // SAFETY: the caller's contract above.
    unsafe { crate::arch::mmio_r32(a) }
}
#[inline]
unsafe fn w32(a: u32, v: u32) {
    // SAFETY: the caller's contract above.
    unsafe { crate::arch::mmio_w32(a, v) }
}

/// The modern interface of a virtio PCI device.
pub struct ModernPci {
    dev: *mut PCIDevice,
    common: u32,
    notify: u32,
    notify_mult: u32,
    isr: u32,
    device_cfg: u32,
    notify_addr: [u32; MAX_QUEUES],
}

impl ModernPci {
    /// Find the modern interface's registers from `dev`'s capabilities.
    ///
    /// The upper half of any 64-bit memory BAR that reads `0xffffffff` is
    /// zeroed first: the AmigaOne's PCI enumeration sizes it and never
    /// writes it back, which puts the BAR outside the bridge's window.
    ///
    /// # Errors
    /// `Device` if the common, notify or ISR capability is missing or its
    /// BAR is not a memory range.
    ///
    /// # Safety
    /// `dev` must be a live PCIDevice the caller has locked.
    pub unsafe fn new(dev: *mut PCIDevice) -> Result<Self> {
        // SAFETY: `dev` is a live, locked PCIDevice (this function's
        // contract); everything below only reads and writes its config space.
        unsafe {
            fix_64bit_bar_high_words(dev);
            if pci!(dev, ReadConfigWord, PCI_STATUS) & PCI_STATUS_CAP_LIST == 0 {
                return Err(AmigaError::Device("no PCI capability list"));
            }
        }
        let (mut common, mut notify, mut isr, mut device_cfg, mut mult) = (0, 0, 0, 0, 0);
        // SAFETY: as above.
        let mut ptr = unsafe { (pci!(dev, ReadConfigByte, PCI_CAP_PTR) & 0xFC) as u32 };
        let mut guard = 0;
        while ptr >= 0x40 && guard < 48 {
            guard += 1;
            // SAFETY: as above; `ptr` is a capability inside config space.
            let (id, next) = unsafe {
                (pci!(dev, ReadConfigByte, ptr), (pci!(dev, ReadConfigByte, ptr + 1) & 0xFC) as u32)
            };
            if id == PCI_CAP_ID_VENDOR {
                // SAFETY: as above; a vendor capability is at least 16 bytes.
                let (cfg_type, bar, offset) = unsafe {
                    (
                        pci!(dev, ReadConfigByte, ptr + 3),
                        pci!(dev, ReadConfigByte, ptr + 4),
                        pci!(dev, ReadConfigLong, ptr + 8),
                    )
                };
                // The first capability of each type is the one to use.
                let slot = match cfg_type {
                    CAP_COMMON if common == 0 => Some(&mut common),
                    CAP_NOTIFY if notify == 0 => Some(&mut notify),
                    CAP_ISR if isr == 0 => Some(&mut isr),
                    CAP_DEVICE if device_cfg == 0 => Some(&mut device_cfg),
                    _ => None,
                };
                if let Some(slot) = slot {
                    // SAFETY: as above.
                    if let Some(base) = unsafe { memory_bar(dev, bar) } {
                        *slot = base + offset;
                        if cfg_type == CAP_NOTIFY {
                            // SAFETY: as above; the notify capability is 20 bytes.
                            mult = unsafe { pci!(dev, ReadConfigLong, ptr + 16) };
                        }
                    }
                }
            }
            ptr = next;
        }
        if common == 0 || notify == 0 || isr == 0 {
            return Err(AmigaError::Device("no modern interface (common, notify or ISR capability missing)"));
        }
        Ok(Self { dev, common, notify, notify_mult: mult, isr, device_cfg, notify_addr: [0; MAX_QUEUES] })
    }

    /// Check the CPU can reach the registers: reset, write ACKNOWLEDGE to
    /// the status register, read it back, reset again. Turns on memory
    /// decoding and bus mastering first.
    ///
    /// # Safety
    /// As [`new`](Self::new); the device is left reset.
    pub unsafe fn probe(&mut self) -> bool {
        // SAFETY: a live, locked device (the contract) whose common
        // configuration `new` found in a memory BAR.
        unsafe {
            enable_pci(self.dev, true);
            w8(self.common + C_STATUS, 0);
            w8(self.common + C_STATUS, STATUS_ACKNOWLEDGE);
            let back = r8(self.common + C_STATUS);
            w8(self.common + C_STATUS, 0);
            back == STATUS_ACKNOWLEDGE
        }
    }

    /// The CPU address of the common configuration.
    #[inline]
    pub fn common_base(&self) -> u32 {
        self.common
    }

    /// The CPU address of the device-specific configuration, 0 if none.
    #[inline]
    pub fn device_cfg(&self) -> u32 {
        self.device_cfg
    }

    /// The PCI device.
    #[inline]
    pub fn device(&self) -> *mut PCIDevice {
        self.dev
    }
}

/// The CPU address of `bar` if it is a memory range.
///
/// # Safety
/// `dev` must be a live PCIDevice the caller has locked.
unsafe fn memory_bar(dev: *mut PCIDevice, bar: u8) -> Option<u32> {
    if bar > 5 {
        return None;
    }
    // SAFETY: a live, locked device (the contract); bar is 0..=5.
    let r = unsafe { pci!(dev, GetResourceRange, bar) };
    if r.is_null() {
        return None;
    }
    // SAFETY: a non-null range from GetResourceRange, read unaligned since
    // the SDK packs the structure.
    let (flags, base) = unsafe {
        (
            core::ptr::addr_of!((*r).Flags).read_unaligned(),
            core::ptr::addr_of!((*r).BaseAddress).read_unaligned(),
        )
    };
    if flags & sys::PCI_RANGE_IO != 0 {
        return None;
    }
    Some(base)
}

/// Zero the upper half of any 64-bit memory BAR left at `0xffffffff`.
///
/// # Safety
/// `dev` must be a live PCIDevice the caller has locked.
unsafe fn fix_64bit_bar_high_words(dev: *mut PCIDevice) {
    let mut i = 0u32;
    while i < 5 {
        // SAFETY: a live, locked device (the contract); BAR registers only.
        let lo = unsafe { pci!(dev, ReadConfigLong, PCI_BAR0 + 4 * i) };
        let is_64bit_memory = lo & 1 == 0 && (lo >> 1) & 3 == 2;
        if is_64bit_memory {
            let hi_reg = PCI_BAR0 + 4 * (i + 1);
            // SAFETY: as above; i < 5, so hi_reg is BAR1..BAR5.
            unsafe {
                if pci!(dev, ReadConfigLong, hi_reg) == 0xFFFF_FFFF {
                    pci!(dev, WriteConfigLong, hi_reg, 0);
                }
            }
            i += 2;
        } else {
            i += 1;
        }
    }
}

// Every block in this impl: the Transport contract (a live, locked device),
// and the register addresses `new` found in its memory BARs.
impl Transport for ModernPci {
    unsafe fn reset(&mut self) -> Result<()> {
        // SAFETY: the Transport contract; common and isr are mapped registers.
        unsafe {
            w8(self.common + C_STATUS, 0);
            for _ in 0..100_000 {
                if r8(self.common + C_STATUS) == 0 {
                    let _ = r8(self.isr);
                    return Ok(());
                }
            }
        }
        Err(AmigaError::Device("reset did not complete"))
    }
    unsafe fn status(&self) -> u8 {
        // SAFETY: the Transport contract; common is a mapped register block.
        unsafe { r8(self.common + C_STATUS) }
    }
    unsafe fn set_status(&mut self, status: u8) {
        // SAFETY: the Transport contract; common is a mapped register block.
        unsafe { w8(self.common + C_STATUS, status) }
    }
    unsafe fn device_features(&mut self) -> u64 {
        // SAFETY: the Transport contract; common is a mapped register block.
        unsafe {
            w32(self.common + C_DFSELECT, 0);
            let lo = r32(self.common + C_DF);
            w32(self.common + C_DFSELECT, 1);
            let hi = r32(self.common + C_DF);
            (hi as u64) << 32 | lo as u64
        }
    }
    unsafe fn set_driver_features(&mut self, features: u64) {
        // SAFETY: the Transport contract; common is a mapped register block.
        unsafe {
            w32(self.common + C_GFSELECT, 0);
            w32(self.common + C_GF, features as u32);
            w32(self.common + C_GFSELECT, 1);
            w32(self.common + C_GF, (features >> 32) as u32);
        }
    }
    unsafe fn queue_max(&mut self, q: u16) -> u16 {
        // SAFETY: the Transport contract; common is a mapped register block.
        unsafe {
            w16(self.common + C_Q_SELECT, q);
            r16(self.common + C_Q_SIZE)
        }
    }
    unsafe fn setup_queue(&mut self, q: u16, vq: &Virtqueue) -> Result<()> {
        if q as usize >= MAX_QUEUES {
            return Err(AmigaError::Device("queue number out of range"));
        }
        // SAFETY: the Transport contract; common is a mapped register block.
        let max = unsafe {
            w16(self.common + C_Q_SELECT, q);
            r16(self.common + C_Q_SIZE)
        };
        let size = vq.layout().size;
        if size == 0 || size > max {
            return Err(AmigaError::Device("queue larger than the device allows"));
        }
        // SAFETY: the Transport contract; common is a mapped register block,
        // and vq's rings stay allocated while the queue is enabled.
        let off = unsafe {
            w16(self.common + C_Q_SIZE, size);
            for (reg, addr) in [(C_Q_DESC, vq.desc_phys()), (C_Q_DRIVER, vq.avail_phys()), (C_Q_DEVICE, vq.used_phys())] {
                w32(self.common + reg, addr);
                w32(self.common + reg + 4, 0);
            }
            w16(self.common + C_Q_ENABLE, 1);
            r16(self.common + C_Q_NOFF) as u32
        };
        self.notify_addr[q as usize] = self.notify + off * self.notify_mult;
        Ok(())
    }
    unsafe fn notify(&mut self, q: u16) {
        if let Some(&a) = self.notify_addr.get(q as usize) {
            if a != 0 {
                // SAFETY: the Transport contract; `a` is the queue's notify
                // register, set by setup_queue.
                unsafe { w16(a, q) }
            }
        }
    }
    unsafe fn read_isr(&self) -> u8 {
        // SAFETY: the Transport contract; isr is a mapped register.
        unsafe { r8(self.isr) }
    }
    unsafe fn config_u8(&self, off: u32) -> u8 {
        if self.device_cfg == 0 {
            0
        } else {
            // SAFETY: the Transport contract, and `off` inside the device
            // configuration (the trait method's own contract).
            unsafe { r8(self.device_cfg + off) }
        }
    }
    fn is_modern(&self) -> bool {
        true
    }
}

/// Interrupt-safe access to a device's ISR and status registers, for an
/// interrupt server or reset callback, which cannot borrow the driver's
/// transport. Copy it out with [`PciTransport::regs`] before adding the
/// server.
#[derive(Clone, Copy)]
pub struct Regs {
    dev: *mut PCIDevice,
    isr: u32,
    status: u32,
    mmio: bool,
}

// SAFETY: a plain description of register addresses; using it is unsafe.
unsafe impl Send for Regs {}
// SAFETY: as Send: it holds no data, only addresses, and every use is unsafe.
unsafe impl Sync for Regs {}

impl Regs {
    /// Read the ISR, which clears it and drops the device's INTx line.
    ///
    /// # Safety
    /// The device must still be the one these addresses describe.
    #[inline]
    pub unsafe fn read_isr(&self) -> u8 {
        // SAFETY: the device is still the one described (the contract).
        unsafe { if self.mmio { r8(self.isr) } else { pci!(self.dev, InByte, self.isr) } }
    }

    /// Write the device status byte (0 resets the device).
    ///
    /// # Safety
    /// As [`read_isr`](Self::read_isr).
    #[inline]
    pub unsafe fn write_status(&self, v: u8) {
        // SAFETY: the device is still the one described (the contract).
        unsafe { if self.mmio { w8(self.status, v) } else { pci!(self.dev, OutByte, self.status, v) } }
    }
}

/// Either interface of a virtio PCI device.
pub enum PciTransport {
    Modern(ModernPci),
    Legacy(LegacyPci),
}

impl PciTransport {
    /// Open `dev`: the modern interface if it has one and the probe passes,
    /// otherwise the legacy interface. Returns the transport and, when it
    /// fell back to legacy, why the modern one was not used.
    ///
    /// # Errors
    /// `Device` if neither interface is usable.
    ///
    /// # Safety
    /// `dev` must be a live PCIDevice the caller has locked.
    pub unsafe fn open(dev: *mut PCIDevice) -> Result<(Self, Option<&'static str>)> {
        // SAFETY: `dev` is a live, locked PCIDevice (this function's contract),
        // which is what new, probe, enable_pci and LegacyPci::new need.
        unsafe {
            let why = match ModernPci::new(dev) {
                Ok(mut m) => {
                    if m.probe() {
                        return Ok((PciTransport::Modern(m), None));
                    }
                    "the modern registers did not answer (the PCI bridge does not forward memory cycles to the BAR)"
                }
                Err(AmigaError::Device(what)) => what,
                Err(_) => "the modern interface could not be read",
            };
            enable_pci(dev, false);
            match LegacyPci::new(dev) {
                Ok(l) => Ok((PciTransport::Legacy(l), Some(why))),
                Err(_) => Err(AmigaError::Device("neither the modern nor the legacy interface is usable")),
            }
        }
    }

    /// Interrupt-safe register access for servers and reset callbacks.
    pub fn regs(&self) -> Regs {
        match self {
            PciTransport::Modern(m) => Regs { dev: m.dev, isr: m.isr, status: m.common + C_STATUS, mmio: true },
            PciTransport::Legacy(l) => Regs { dev: l.device(), isr: l.isr_port(), status: l.io_base() + 0x12, mmio: false },
        }
    }

    /// The PCI device.
    pub fn device(&self) -> *mut PCIDevice {
        match self {
            PciTransport::Modern(m) => m.dev,
            PciTransport::Legacy(l) => l.device(),
        }
    }

    /// A way to notify queue `q` from another task, without the transport
    /// (which its owner keeps): see [`Notifier`]. None if the queue is not
    /// set up (modern interface).
    pub fn notifier(&self, q: u16) -> Option<Notifier> {
        match self {
            PciTransport::Modern(m) => {
                let a = *m.notify_addr.get(q as usize)?;
                if a == 0 { None } else { Some(Notifier { dev: m.dev, addr: a, q, mmio: true }) }
            }
            PciTransport::Legacy(l) => Some(Notifier { dev: l.device(), addr: l.io_base() + 0x10, q, mmio: false }),
        }
    }
}

/// Notifies one queue: its notify register, copied out of the transport so
/// a task other than the transport's owner can kick the device after it
/// posts buffers (under the queue's own lock).
#[derive(Clone, Copy)]
pub struct Notifier {
    dev: *mut PCIDevice,
    addr: u32,
    q: u16,
    mmio: bool,
}

// SAFETY: a register address; writes to it are what a notify is.
unsafe impl Send for Notifier {}

impl Notifier {
    /// Tell the device the queue has new buffers.
    ///
    /// # Safety
    /// The device and queue must still be the ones this was taken from.
    #[inline]
    pub unsafe fn notify(&self) {
        // SAFETY: the device and queue are the ones this was taken from (the
        // contract).
        unsafe { if self.mmio { w16(self.addr, self.q) } else { pci!(self.dev, OutWord, self.addr, self.q) } }
    }
}

macro_rules! both {
    ($s:expr, $t:ident => $e:expr) => {
        match $s {
            PciTransport::Modern($t) => $e,
            PciTransport::Legacy($t) => $e,
        }
    };
}

// Each method passes its own Transport contract on to the interface it holds.
impl Transport for PciTransport {
    unsafe fn reset(&mut self) -> Result<()> {
        // SAFETY: this method's contract, passed on.
        unsafe { both!(self, t => t.reset()) }
    }
    unsafe fn status(&self) -> u8 {
        // SAFETY: this method's contract, passed on.
        unsafe { both!(self, t => t.status()) }
    }
    unsafe fn set_status(&mut self, status: u8) {
        // SAFETY: this method's contract, passed on.
        unsafe { both!(self, t => t.set_status(status)) }
    }
    unsafe fn device_features(&mut self) -> u64 {
        // SAFETY: this method's contract, passed on.
        unsafe { both!(self, t => t.device_features()) }
    }
    unsafe fn set_driver_features(&mut self, features: u64) {
        // SAFETY: this method's contract, passed on.
        unsafe { both!(self, t => t.set_driver_features(features)) }
    }
    unsafe fn queue_max(&mut self, q: u16) -> u16 {
        // SAFETY: this method's contract, passed on.
        unsafe { both!(self, t => t.queue_max(q)) }
    }
    unsafe fn setup_queue(&mut self, q: u16, vq: &Virtqueue) -> Result<()> {
        // SAFETY: this method's contract, passed on.
        unsafe { both!(self, t => t.setup_queue(q, vq)) }
    }
    unsafe fn notify(&mut self, q: u16) {
        // SAFETY: this method's contract, passed on.
        unsafe { both!(self, t => t.notify(q)) }
    }
    unsafe fn read_isr(&self) -> u8 {
        // SAFETY: this method's contract, passed on.
        unsafe { both!(self, t => t.read_isr()) }
    }
    unsafe fn config_u8(&self, off: u32) -> u8 {
        // SAFETY: this method's contract, passed on.
        unsafe { both!(self, t => t.config_u8(off)) }
    }
    fn is_modern(&self) -> bool {
        matches!(self, PciTransport::Modern(_))
    }
}

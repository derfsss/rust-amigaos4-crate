//! How a driver talks to a virtio device's registers.
//!
//! [`Transport`] is what a driver needs from the device besides its
//! queues: reset and status, feature negotiation, giving it a queue,
//! notifying it, and its interrupt status. [`LegacyPci`] is the legacy
//! interface (I/O BAR 0); `ModernPci` is the modern one (target only: it
//! uses the CPU's MMIO instructions). [`init_device`] and [`driver_ok`]
//! run the standard initialisation sequence against either.

use super::ring::Endian;
use super::queue::Virtqueue;
use super::{
    STATUS_ACKNOWLEDGE, STATUS_DRIVER, STATUS_DRIVER_OK, STATUS_FAILED, STATUS_FEATURES_OK,
    F_VERSION_1,
};
use crate::error::{AmigaError, Result};
use crate::sys::{self, PCIDevice};

macro_rules! pci {
    ($dev:expr, $method:ident $(, $arg:expr)*) => {
        ((*$dev).$method)($dev $(, $arg)*)
    };
}
pub(crate) use pci;

/// A virtio device's register interface.
///
/// # Safety
///
/// Every `unsafe` method has one contract: the transport still addresses
/// the live device it was made for (its PCIDevice locked by the driver, its
/// BARs mapped), and the driver serialises register access (one task at a
/// time, or an interrupt server that only calls [`read_isr`](Self::read_isr)).
pub trait Transport {
    /// Write status 0 (reset) and wait until the device reads back 0.
    ///
    /// # Safety
    /// The trait's contract.
    unsafe fn reset(&mut self) -> Result<()>;
    /// The device status byte.
    ///
    /// # Safety
    /// The trait's contract.
    unsafe fn status(&self) -> u8;
    /// Write the device status byte.
    ///
    /// # Safety
    /// The trait's contract.
    unsafe fn set_status(&mut self, status: u8);
    /// The features the device offers (a legacy device has 32 bits).
    ///
    /// # Safety
    /// The trait's contract.
    unsafe fn device_features(&mut self) -> u64;
    /// The features the driver accepts.
    ///
    /// # Safety
    /// The trait's contract.
    unsafe fn set_driver_features(&mut self, features: u64);
    /// The largest size queue `q` allows; 0 means there is no such queue.
    ///
    /// # Safety
    /// The trait's contract.
    unsafe fn queue_max(&mut self, q: u16) -> u16;
    /// Give queue `q` to the device and enable it.
    ///
    /// # Safety
    /// The trait's contract.
    unsafe fn setup_queue(&mut self, q: u16, vq: &Virtqueue) -> Result<()>;
    /// Tell the device queue `q` has new buffers.
    ///
    /// # Safety
    /// The trait's contract.
    unsafe fn notify(&mut self, q: u16);
    /// Read the interrupt status, which also clears it and drops the
    /// device's INTx line. Safe in an interrupt server.
    ///
    /// # Safety
    /// The trait's contract.
    unsafe fn read_isr(&self) -> u8;
    /// Read byte `off` of the device-specific configuration.
    ///
    /// # Safety
    /// The trait's contract, and `off` must be inside the device's
    /// configuration.
    unsafe fn config_u8(&self, off: u32) -> u8;
    /// True for the modern (VIRTIO_F_VERSION_1) interface.
    fn is_modern(&self) -> bool;
    /// Read a 16-bit configuration field, from bytes, in the interface's
    /// byte order: little-endian on the modern interface, the CPU's on the
    /// legacy one.
    ///
    /// # Safety
    /// As for `config_u8`.
    unsafe fn config_u16(&self, off: u32) -> u16 {
        // SAFETY: this method's contract (the trait's, and `off` inside the
        // configuration).
        unsafe {
            let b = [self.config_u8(off), self.config_u8(off + 1)];
            if self.is_modern() { u16::from_le_bytes(b) } else { u16::from_ne_bytes(b) }
        }
    }
    /// Read a 32-bit configuration field, as `config_u16`.
    ///
    /// # Safety
    /// As for `config_u8`.
    unsafe fn config_u32(&self, off: u32) -> u32 {
        // SAFETY: this method's contract (the trait's, and `off` inside the
        // configuration).
        unsafe {
            let b = [self.config_u8(off), self.config_u8(off + 1), self.config_u8(off + 2), self.config_u8(off + 3)];
            if self.is_modern() { u32::from_le_bytes(b) } else { u32::from_ne_bytes(b) }
        }
    }
    /// The byte order of this interface's rings.
    fn ring_endian(&self) -> Endian {
        if self.is_modern() { Endian::Little } else { Endian::Native }
    }
}

/// Reset the device and negotiate features (virtio 1.x section 3.1.1,
/// steps 1 to 6): reset, ACKNOWLEDGE, DRIVER, read the offered features,
/// accept `wanted` of them (plus VIRTIO_F_VERSION_1 on the modern
/// interface), and on the modern interface set FEATURES_OK and check the
/// device kept it. Returns the features accepted. The queues are set up
/// next, then [`driver_ok`].
///
/// # Safety
/// `t` must address a live device.
pub unsafe fn init_device<T: Transport>(t: &mut T, wanted: u64) -> Result<u64> {
    // SAFETY: `t` addresses a live device (this function's contract).
    unsafe {
        t.reset()?;
        t.set_status(STATUS_ACKNOWLEDGE);
        t.set_status(STATUS_ACKNOWLEDGE | STATUS_DRIVER);
        let offered = t.device_features();
        let mut accepted = offered & wanted;
        if t.is_modern() {
            if offered & F_VERSION_1 == 0 {
                t.set_status(STATUS_FAILED);
                return Err(AmigaError::Device("modern interface without VIRTIO_F_VERSION_1"));
            }
            accepted |= F_VERSION_1;
        }
        t.set_driver_features(accepted);
        if t.is_modern() {
            t.set_status(STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK);
            if t.status() & STATUS_FEATURES_OK == 0 {
                t.set_status(STATUS_FAILED);
                return Err(AmigaError::Device("device refused FEATURES_OK"));
            }
        }
        Ok(accepted)
    }
}

/// Set DRIVER_OK once the queues are set up, and check the device kept it.
///
/// # Safety
/// `t` must address a live device.
pub unsafe fn driver_ok<T: Transport>(t: &mut T) -> Result<()> {
    // SAFETY: `t` addresses a live device (this function's contract).
    unsafe {
        let s = t.status();
        t.set_status(s | STATUS_DRIVER_OK);
        if t.status() & STATUS_DRIVER_OK == 0 {
            return Err(AmigaError::Device("device refused DRIVER_OK"));
        }
        Ok(())
    }
}

// ---- The legacy interface: registers in I/O BAR 0 ----

const LEGACY_HOST_FEATURES: u32 = 0x00;
const LEGACY_GUEST_FEATURES: u32 = 0x04;
const LEGACY_QUEUE_PFN: u32 = 0x08;
const LEGACY_QUEUE_NUM: u32 = 0x0C;
const LEGACY_QUEUE_SEL: u32 = 0x0E;
const LEGACY_QUEUE_NOTIFY: u32 = 0x10;
const LEGACY_STATUS: u32 = 0x12;
const LEGACY_ISR: u32 = 0x13;
/// Where device-specific configuration starts (no MSI-X).
pub const LEGACY_DEVICE_CONFIG: u32 = 0x14;

/// The legacy interface of a transitional virtio PCI device: registers in
/// I/O BAR 0, accessed through the PCIDevice's I/O methods, rings in the
/// CPU's byte order, each queue given as one page number.
///
/// Works behind every PCI bridge AmigaOS 4 runs on, including the
/// AmigaOne's Articia S, where the modern interface's memory BAR does not.
pub struct LegacyPci {
    dev: *mut PCIDevice,
    io: u32,
}

impl LegacyPci {
    /// The legacy interface of `dev`, whose BAR 0 must be I/O space.
    ///
    /// # Safety
    /// `dev` must be a live PCIDevice the caller has locked.
    pub unsafe fn new(dev: *mut PCIDevice) -> Result<Self> {
        // SAFETY: `dev` is a live, locked PCIDevice (the contract), and its
        // resource range is read where the SDK puts it.
        unsafe {
            let bar0 = pci!(dev, GetResourceRange, 0);
            if bar0.is_null() {
                return Err(AmigaError::Device("no BAR 0"));
            }
            let flags = core::ptr::addr_of!((*bar0).Flags).read_unaligned();
            if flags & sys::PCI_RANGE_IO == 0 {
                return Err(AmigaError::Device("BAR 0 is not I/O space; no legacy interface"));
            }
            let io = core::ptr::addr_of!((*bar0).Physical).read_unaligned();
            Ok(Self { dev, io })
        }
    }

    /// The I/O base of BAR 0.
    #[inline]
    pub fn io_base(&self) -> u32 {
        self.io
    }

    /// The I/O address of the ISR register, for an interrupt server that
    /// reads it directly.
    #[inline]
    pub fn isr_port(&self) -> u32 {
        self.io + LEGACY_ISR
    }

    /// The PCI device.
    #[inline]
    pub fn device(&self) -> *mut PCIDevice {
        self.dev
    }

}

impl Transport for LegacyPci {
    unsafe fn reset(&mut self) -> Result<()> {
        // SAFETY: the Transport contract: a live, locked device whose I/O BAR 0
        // is at self.io.
        unsafe {
            pci!(self.dev, OutByte, self.io + LEGACY_STATUS, 0u8);
            for _ in 0..100_000 {
                if pci!(self.dev, InByte, self.io + LEGACY_STATUS) == 0 {
                    let _ = pci!(self.dev, InByte, self.io + LEGACY_ISR);
                    return Ok(());
                }
            }
            Err(AmigaError::Device("reset did not complete"))
        }
    }
    unsafe fn status(&self) -> u8 {
        // SAFETY: the Transport contract: a live, locked device whose I/O BAR 0
        // is at self.io.
        unsafe {
            pci!(self.dev, InByte, self.io + LEGACY_STATUS)
        }
    }
    unsafe fn set_status(&mut self, status: u8) {
        // SAFETY: the Transport contract: a live, locked device whose I/O BAR 0
        // is at self.io.
        unsafe {
            pci!(self.dev, OutByte, self.io + LEGACY_STATUS, status);
        }
    }
    unsafe fn device_features(&mut self) -> u64 {
        // SAFETY: the Transport contract: a live, locked device whose I/O BAR 0
        // is at self.io.
        unsafe {
            pci!(self.dev, InLong, self.io + LEGACY_HOST_FEATURES) as u64
        }
    }
    unsafe fn set_driver_features(&mut self, features: u64) {
        // SAFETY: the Transport contract: a live, locked device whose I/O BAR 0
        // is at self.io.
        unsafe {
            pci!(self.dev, OutLong, self.io + LEGACY_GUEST_FEATURES, features as u32);
        }
    }
    unsafe fn queue_max(&mut self, q: u16) -> u16 {
        // SAFETY: the Transport contract: a live, locked device whose I/O BAR 0
        // is at self.io.
        unsafe {
            pci!(self.dev, OutWord, self.io + LEGACY_QUEUE_SEL, q);
            pci!(self.dev, InWord, self.io + LEGACY_QUEUE_NUM)
        }
    }
    unsafe fn setup_queue(&mut self, q: u16, vq: &Virtqueue) -> Result<()> {
        // SAFETY: the Transport contract: a live, locked device whose I/O BAR 0
        // is at self.io.
        unsafe {
            pci!(self.dev, OutWord, self.io + LEGACY_QUEUE_SEL, q);
            // The legacy interface has no queue size register to write: the
            // queue must be the size the device reports.
            if pci!(self.dev, InWord, self.io + LEGACY_QUEUE_NUM) != vq.layout().size {
                return Err(AmigaError::Device("a legacy queue must be the size the device reports"));
            }
            pci!(self.dev, OutLong, self.io + LEGACY_QUEUE_PFN, vq.desc_phys() / 4096);
            Ok(())
        }
    }
    unsafe fn notify(&mut self, q: u16) {
        // SAFETY: the Transport contract: a live, locked device whose I/O BAR 0
        // is at self.io.
        unsafe {
            pci!(self.dev, OutWord, self.io + LEGACY_QUEUE_NOTIFY, q);
        }
    }
    unsafe fn read_isr(&self) -> u8 {
        // SAFETY: the Transport contract: a live, locked device whose I/O BAR 0
        // is at self.io.
        unsafe {
            pci!(self.dev, InByte, self.io + LEGACY_ISR)
        }
    }
    unsafe fn config_u8(&self, off: u32) -> u8 {
        // SAFETY: the Transport contract: a live, locked device whose I/O BAR 0
        // is at self.io.
        unsafe {
            pci!(self.dev, InByte, self.io + LEGACY_DEVICE_CONFIG + off)
        }
    }
    fn is_modern(&self) -> bool {
        false
    }
}

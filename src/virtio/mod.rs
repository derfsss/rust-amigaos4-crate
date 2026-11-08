//! virtio devices over PCI (feature `virtio`).
//!
//! The parts every virtio driver needs and that do not depend on the device
//! type:
//!
//! - [`ring`]: the split virtqueue's layout and its descriptor, avail and
//!   used operations over raw memory, in either byte order, with the cache
//!   maintenance around the device's accesses;
//! - [`Virtqueue`]: a ring in one contiguous block of DMA memory, mapped
//!   for the device for its lifetime;
//! - [`Transport`]: the device's registers (reset, status, features, queue
//!   setup, notify, interrupt status), with [`LegacyPci`] for the legacy
//!   interface, `ModernPci` for the modern one, and `PciTransport`, which
//!   opens whichever works (modern first, legacy as the fallback);
//!   [`init_device`] / [`driver_ok`] run the standard initialisation
//!   sequence against any of them;
//! - the status and feature bits, the PCI IDs, and [`enable_pci`].
//!
//! The device-specific part (what goes in the buffers, how requests are
//! served) stays in the driver.

pub mod net;
pub mod scsi;
pub mod ring;
mod queue;
mod transport;
#[cfg(target_os = "amigaos")]
mod modern;

pub use queue::Virtqueue;
pub use ring::{Endian, RingLayout, SplitRing};
pub use transport::{driver_ok, init_device, LegacyPci, Transport, LEGACY_DEVICE_CONFIG};
#[cfg(target_os = "amigaos")]
pub use modern::{ModernPci, Notifier, PciTransport, Regs};

use crate::sys::PCIDevice;
use transport::pci;

/// PCI vendor ID of every virtio device.
pub const PCI_VENDOR: u32 = 0x1AF4;

/// Device status bits (virtio 1.x section 2.1).
pub const STATUS_ACKNOWLEDGE: u8 = 1;
pub const STATUS_DRIVER: u8 = 2;
pub const STATUS_DRIVER_OK: u8 = 4;
pub const STATUS_FEATURES_OK: u8 = 8;
pub const STATUS_DEVICE_NEEDS_RESET: u8 = 64;
pub const STATUS_FAILED: u8 = 128;

/// The device complies with virtio 1.x (the modern interface).
pub const F_VERSION_1: u64 = 1 << 32;
/// Descriptors may point at tables of descriptors (ring::DESC_F_INDIRECT).
pub const F_INDIRECT_DESC: u64 = 1 << 28;
/// The used_event / avail_event fields suppress notifications.
pub const F_EVENT_IDX: u64 = 1 << 29;

/// ISR bits: a queue was used; the configuration changed.
pub const ISR_QUEUE: u8 = 1;
pub const ISR_CONFIG: u8 = 2;

/// Device types (virtio 1.x section 5).
pub const TYPE_NET: u16 = 1;
pub const TYPE_BLOCK: u16 = 2;
pub const TYPE_CONSOLE: u16 = 3;
pub const TYPE_ENTROPY: u16 = 4;
pub const TYPE_SCSI: u16 = 8;
pub const TYPE_9P: u16 = 9;

/// The PCI device ID of a modern-only device of `device_type`
/// (0x1040 + type).
pub const fn pci_id_modern(device_type: u16) -> u32 {
    0x1040 + device_type as u32
}

/// The PCI device ID of a transitional device of `device_type` (it offers
/// both interfaces), or `None` for a type that has no transitional ID.
pub const fn pci_id_transitional(device_type: u16) -> Option<u32> {
    match device_type {
        TYPE_NET => Some(0x1000),
        TYPE_BLOCK => Some(0x1001),
        TYPE_CONSOLE => Some(0x1003),
        TYPE_ENTROPY => Some(0x1005),
        TYPE_SCSI => Some(0x1004),
        TYPE_9P => Some(0x1009),
        _ => None,
    }
}

const PCI_COMMAND: u32 = 0x04;
const PCI_COMMAND_IO: u16 = 1 << 0;
const PCI_COMMAND_MEMORY: u16 = 1 << 1;
const PCI_COMMAND_MASTER: u16 = 1 << 2;

/// Turn on bus mastering and I/O decoding (and memory decoding when
/// `memory` is true) in the PCI command register, which firmware may leave
/// off (the Pegasos II does). Returns the register before and after.
///
/// # Safety
/// `dev` must be a live PCIDevice the caller has locked.
pub unsafe fn enable_pci(dev: *mut PCIDevice, memory: bool) -> (u16, u16) {
    // SAFETY: `dev` is a live PCIDevice the caller has locked (the contract).
    unsafe {
        let old = pci!(dev, ReadConfigWord, PCI_COMMAND);
        let want = old | PCI_COMMAND_IO | PCI_COMMAND_MASTER | if memory { PCI_COMMAND_MEMORY } else { 0 };
        if want != old {
            pci!(dev, WriteConfigWord, PCI_COMMAND, want);
        }
        (old, pci!(dev, ReadConfigWord, PCI_COMMAND))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pci_ids() {
        assert_eq!(pci_id_modern(TYPE_ENTROPY), 0x1044);
        assert_eq!(pci_id_transitional(TYPE_ENTROPY), Some(0x1005));
        assert_eq!(pci_id_transitional(TYPE_9P), Some(0x1009));
        assert_eq!(pci_id_transitional(42), None);
    }
}

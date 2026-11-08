//! virtio-net (virtio 1.x section 5.1): feature bits, the device
//! configuration layout, the header before every packet, and the control
//! queue's commands.
//!
//! Queues: 0 receive, 1 transmit, and (with `F_CTRL_VQ`) 2 control, when
//! there is one queue pair (no `F_MQ`).

/// PCI device ID, transitional (legacy and modern interfaces).
pub const PCI_ID_TRANSITIONAL: u32 = 0x1000;
/// PCI device ID, modern only.
pub const PCI_ID_MODERN: u32 = 0x1041;

pub const RX_QUEUE: u16 = 0;
pub const TX_QUEUE: u16 = 1;
/// The control queue's index with one queue pair.
pub const CTRL_QUEUE: u16 = 2;

// ---- Feature bits (section 5.1.3) ----

/// The device handles packets with partial checksum (offload).
pub const F_CSUM: u64 = 1 << 0;
/// The driver handles packets with partial checksum.
pub const F_GUEST_CSUM: u64 = 1 << 1;
pub const F_CTRL_GUEST_OFFLOADS: u64 = 1 << 2;
/// The device reports its maximum MTU in the configuration.
pub const F_MTU: u64 = 1 << 3;
/// The device has a MAC address in the configuration.
pub const F_MAC: u64 = 1 << 5;
pub const F_GUEST_TSO4: u64 = 1 << 7;
pub const F_GUEST_TSO6: u64 = 1 << 8;
pub const F_GUEST_ECN: u64 = 1 << 9;
pub const F_GUEST_UFO: u64 = 1 << 10;
pub const F_HOST_TSO4: u64 = 1 << 11;
pub const F_HOST_TSO6: u64 = 1 << 12;
pub const F_HOST_ECN: u64 = 1 << 13;
pub const F_HOST_UFO: u64 = 1 << 14;
/// The driver can merge receive buffers (the header has num_buffers).
pub const F_MRG_RXBUF: u64 = 1 << 15;
/// The configuration's status field is valid (link status).
pub const F_STATUS: u64 = 1 << 16;
/// There is a control queue.
pub const F_CTRL_VQ: u64 = 1 << 17;
/// The control queue takes receive-mode commands (promiscuous, all multicast).
pub const F_CTRL_RX: u64 = 1 << 18;
/// The control queue takes VLAN filtering commands.
pub const F_CTRL_VLAN: u64 = 1 << 19;
/// The driver can send gratuitous packets when asked (after migration).
pub const F_GUEST_ANNOUNCE: u64 = 1 << 21;
/// Several queue pairs.
pub const F_MQ: u64 = 1 << 22;
/// The MAC address can be set through the control queue.
pub const F_CTRL_MAC_ADDR: u64 = 1 << 23;
/// The configuration reports speed and duplex.
pub const F_SPEED_DUPLEX: u64 = 1 << 63;

// ---- Device configuration (section 5.1.4), byte offsets ----

pub const CFG_MAC: u32 = 0; // u8[6]
pub const CFG_STATUS: u32 = 6; // le16 (F_STATUS)
pub const CFG_MAX_VQ_PAIRS: u32 = 8; // le16 (F_MQ)
pub const CFG_MTU: u32 = 10; // le16 (F_MTU)
pub const CFG_SPEED: u32 = 12; // le32, Mbit/s (F_SPEED_DUPLEX); 0xffffffff unknown
pub const CFG_DUPLEX: u32 = 16; // u8: 0 half, 1 full, 0xff unknown

/// Status: the link is up.
pub const S_LINK_UP: u16 = 1;
/// Status: the driver should announce itself (send gratuitous packets).
pub const S_ANNOUNCE: u16 = 2;

/// Speed and duplex values meaning "unknown".
pub const SPEED_UNKNOWN: u32 = 0xffff_ffff;
pub const DUPLEX_UNKNOWN: u8 = 0xff;

// ---- The header before every packet (section 5.1.6) ----

/// `struct virtio_net_hdr` without `num_buffers`: legacy interface without
/// `F_MRG_RXBUF`.
pub const HDR_LEN_LEGACY: usize = 10;
/// With `num_buffers`: the modern interface, or `F_MRG_RXBUF`.
pub const HDR_LEN: usize = 12;

/// The header length for an interface and features.
pub const fn hdr_len(modern: bool, features: u64) -> usize {
    if modern || features & F_MRG_RXBUF != 0 { HDR_LEN } else { HDR_LEN_LEGACY }
}

/// flags: the checksum is partial (offload); unused without F_CSUM.
pub const HDR_F_NEEDS_CSUM: u8 = 1;
/// flags: the checksum was validated (F_GUEST_CSUM).
pub const HDR_F_DATA_VALID: u8 = 2;
/// gso_type: not a segmentation-offload packet.
pub const HDR_GSO_NONE: u8 = 0;

/// `num_buffers` of a received header (merged receive buffers), read in the
/// interface's byte order; 1 when the header has no such field.
pub fn rx_num_buffers(hdr: &[u8], modern: bool, features: u64) -> u16 {
    if hdr_len(modern, features) < HDR_LEN || hdr.len() < HDR_LEN {
        return 1;
    }
    let b = [hdr[10], hdr[11]];
    let n = if modern { u16::from_le_bytes(b) } else { u16::from_ne_bytes(b) };
    n.max(1)
}

// ---- The control queue (section 5.1.6.5) ----
//
// A command is: class (u8), command (u8), command-specific data (driver to
// device), then one byte the device writes back: CTRL_OK or CTRL_ERR.

pub const CTRL_OK: u8 = 0;
pub const CTRL_ERR: u8 = 1;

/// Receive mode (F_CTRL_RX): data is one byte, 0 off or 1 on.
pub const CTRL_RX: u8 = 0;
pub const CTRL_RX_PROMISC: u8 = 0;
pub const CTRL_RX_ALLMULTI: u8 = 1;

/// MAC filtering.
pub const CTRL_MAC: u8 = 1;
/// The unicast and multicast filter tables: two `struct virtio_net_ctrl_mac`
/// (le32 entries, then entries * 6 bytes), unicast first.
pub const CTRL_MAC_TABLE_SET: u8 = 0;
/// The MAC address (F_CTRL_MAC_ADDR): 6 bytes.
pub const CTRL_MAC_ADDR_SET: u8 = 1;

/// Acknowledge an announce request (F_GUEST_ANNOUNCE).
pub const CTRL_ANNOUNCE: u8 = 3;
pub const CTRL_ANNOUNCE_ACK: u8 = 0;

/// The data of CTRL_MAC_TABLE_SET for these unicast and multicast
/// addresses: each table's entry count (le32 on the modern interface, the
/// CPU's order on the legacy one) followed by its addresses.
pub fn mac_table(unicast: &[[u8; 6]], multicast: &[[u8; 6]], modern: bool) -> alloc::vec::Vec<u8> {
    let mut v = alloc::vec::Vec::with_capacity(8 + 6 * (unicast.len() + multicast.len()));
    for table in [unicast, multicast] {
        let n = table.len() as u32;
        v.extend_from_slice(&if modern { n.to_le_bytes() } else { n.to_ne_bytes() });
        for a in table {
            v.extend_from_slice(a);
        }
    }
    v
}

// ---- Ethernet ----

pub const ETH_ALEN: usize = 6;
pub const ETH_HLEN: usize = 14;
/// Payload MTU when the device does not report one.
pub const ETH_MTU: usize = 1500;
/// Header, payload and one VLAN tag, without the FCS.
pub const ETH_FRAME_MAX: usize = ETH_HLEN + ETH_MTU + 4;
pub const BROADCAST: [u8; 6] = [0xff; 6];

/// Multicast: the group bit (least significant bit of the first byte).
pub const fn is_multicast(addr: &[u8; 6]) -> bool {
    addr[0] & 1 != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_lengths() {
        assert_eq!(hdr_len(false, 0), 10);
        assert_eq!(hdr_len(false, F_MRG_RXBUF), 12);
        assert_eq!(hdr_len(true, 0), 12);
    }

    #[test]
    fn num_buffers_in_each_order() {
        let mut h = [0u8; 12];
        h[10] = 3; // little-endian 3
        assert_eq!(rx_num_buffers(&h, true, 0), 3);
        assert_eq!(rx_num_buffers(&h[..10], false, 0), 1);
        let ne = 2u16.to_ne_bytes();
        h[10] = ne[0];
        h[11] = ne[1];
        assert_eq!(rx_num_buffers(&h, false, F_MRG_RXBUF), 2);
        assert_eq!(rx_num_buffers(&[0u8; 12], true, 0), 1); // 0 read as 1
    }

    #[test]
    fn mac_table_layout() {
        let u = [[0x52, 0x54, 0, 0x12, 0x34, 0x57]];
        let m = [[0x01, 0, 0x5e, 0, 0, 1], [0x33, 0x33, 0, 0, 0, 1]];
        let t = mac_table(&u, &m, true);
        assert_eq!(t.len(), 4 + 6 + 4 + 12);
        assert_eq!(&t[0..4], &1u32.to_le_bytes());
        assert_eq!(&t[4..10], &u[0]);
        assert_eq!(&t[10..14], &2u32.to_le_bytes());
        assert_eq!(&t[14..20], &m[0]);
        assert!(is_multicast(&m[1]) && !is_multicast(&u[0]));
    }
}

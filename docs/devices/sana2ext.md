# SANA-II extensions (unofficial), version 1

Commands, structures and item types that let a SANA-II network driver and a
protocol stack use what modern network cards offer: several frames per
request, checksums checked or computed by the card, and segmentation of
large sends. The C header is `include/devices/sana2ext.h`; the Rust module
is `amigaos4::sana2ext` (feature `sana2`).

These extensions are **not** part of SANA-II Revision 7 or of the AmigaOS 4
SDK. They are designed so that they never get in the way of plain SANA-II:

- a driver that does not have them answers `IOERR_NOCMD`, and the stack
  carries on with `CMD_READ` / `CMD_WRITE`;
- a stack that does not use them never sends them, and a driver that has
  them works as before;
- nothing in the standard `IOSana2Req` changes.

## Why

On a machine where every received frame costs a request, a reply and a copy,
the per-frame overhead sets the speed long before the wire does. Batching
lets one request carry many frames; checksum offload removes the stack's
checksum work on every frame; segmentation lets the stack hand the card one
large send instead of dozens of frames.

## Conventions

The design follows patterns already in the SDK:

- **Sized structures.** Every structure starts or ends with its size
  (`s2xq_Size`, `s2xf_Size`, `s2xb_Size`), as `struct Sana2Connection` and
  the SANA-II hook messages do. A reader uses only the fields that size
  covers; a newer version adds fields at the end.
- **Typed, sized items.** Capabilities and per-frame extras are items: a
  common header (`struct S2XItem`: a `MinNode`, a type and the item's size)
  embedded as the first member of each item, linked in a `MinList`, as
  `expansion/pci.h`'s `struct PCICapability`. A reader walks the list, uses
  the types it knows and steps over the others.
- **A pointer to the standard request.** A frame (`struct S2XFrame`) points
  to an ordinary `IOSana2Req` that describes it, as
  `struct Sana2ConnectionMessage` points to its requests. Everything SANA-II
  already says about a frame (packet type, addresses, length, the buffer
  management cookie, flags, errors) stays in that request.
- **The command's structure in `ios2_StatData`**, as for
  `S2_GETGLOBALSTATS`.

## Portability

The extensions must keep working if AmigaOS moves to 64-bit pointers,
several CPUs, or another CPU (little-endian, for example). So:

- **Layout.** Every field lies at an offset that is a multiple of its own
  size, and every structure is a multiple of the pointer size long. The
  layout is therefore the same with the SDK's `pack(2)` and with natural
  alignment, with 32- and 64-bit pointers. Reserved fields (always 0) pad
  where needed. The header's `_Static_assert`s and
  `tests/layout/sana2ext.c` state the sizes as formulas of the pointer size;
  they are checked with the AmigaOS compiler and with x86-64 and PowerPC
  Linux compilers, and the Rust module checks the same formulas on every
  target it builds for.
- **Types.** Explicit widths only (`uint32`, `uint16`, pointers). Sizes and
  lengths are 32-bit.
- **Byte order.** Every structure field is in the host's byte order. Frame
  data is as on the wire (network order). `S2X_MAGIC` is compared as an
  integer, never as bytes. A checksum's starting value that the stack puts
  into a frame (`S2XFRAME_CHECKSUM` on send) is in network order, as on the
  wire.
- **Several CPUs.** Exec's message passing is the only hand-over between a
  driver and a stack: the driver writes everything in a batch before it
  replies and touches nothing after; the stack reads nothing before it has
  the reply (WaitIO / GetMsg). The capability list a driver returns never
  changes between OpenDevice and CloseDevice, so any task may read it. A
  driver protects its own state with mutexes or semaphores, never
  Forbid/Disable, and reaches frame buffers only through the copy hooks and
  the DMA calls (StartDMA / GetDMAList / EndDMA), never by assuming caches
  are coherent.

## Discovery

1. `NSCMD_DEVICEQUERY`: a driver that has the extensions lists `S2X_QUERY`,
   and the batch commands it supports, in `SupportedCommands`.
2. `S2X_QUERY` (0xC100): `ios2_StatData` points to a `struct S2XQuery`
   whose `s2xq_Size` the caller has set. The driver fills what fits:
   `s2xq_Magic` (`S2X_MAGIC`), `s2xq_Version` (the version it implements),
   `s2xq_Revision` (its own), `s2xq_Features` (one bit per capability, for a
   quick test) and `s2xq_Capabilities`, a pointer to its own read-only list
   of capability items.

A stack trusts the extensions only when the command succeeds and the magic
matches. Any other answer means plain SANA-II.

| Capability | Item | Says |
|---|---|---|
| `S2XCAP_BATCH` | `S2XCapBatch` | the batch commands work; the most frames per batch |
| `S2XCAP_RXCHECKSUM` | `S2XCapChecksum` | which checksums the card checks on receive |
| `S2XCAP_TXCHECKSUM` | `S2XCapChecksum` | which checksums the card computes on send |
| `S2XCAP_SEGMENT` | `S2XCapSegment` | TCP segmentation: protocols, largest send, most segments, smallest segment size |

## Batches

`S2X_READBATCH` (0xC101) and `S2X_WRITEBATCH` (0xC102): `ios2_StatData`
points to a `struct S2XBatch`, a list of `struct S2XFrame`. Each frame
points to its own `IOSana2Req`, an opened copy of the batch's request (same
device, unit and buffer management), and carries a list of frame items (may
be empty).

**Ownership.** While the batch's request is in flight the driver owns the
batch, its frames, their requests and their items. A frame's request is
never sent on its own while it is in a batch, and is never replied on its
own.

**READBATCH.** The batch request's `ios2_PacketType` and `io_Flags`
(`SANA2IOF_RAW`) apply to every frame. The driver fills frames in list
order, as many as are waiting, and at least one: like `CMD_READ`, the request
waits for the first frame. For each frame filled it sets the frame
request's fields as `CMD_READ` would (packet type, addresses, length, the
data through the buffer management, `SANA2IOF_BCAST` / `MCAST`, `io_Error`)
and its items it knows (checksum results). It sets `s2xb_Done` to the number
of frames filled, then replies the batch.

**WRITEBATCH.** Each frame's request is set as for `CMD_WRITE`. The driver
sends frames in list order and counts those it took in `s2xb_Done`. At the
first frame it refuses it stops: that frame's request has the error
(`io_Error`, `ios2_WireError`), and so does the batch's request.

**Abort.** `AbortIO` is called on the batch's request only. A read batch
comes back with `IOERR_ABORTED` and whatever frames are filled (counted in
`s2xb_Done`); a write batch with the frames already taken counted.

**Allocation.** The stack allocates batches, frames and items once (shared
memory: `MEMF_SHARED`) and reuses them; the driver never allocates or frees
any of them. The stack keeps a frame's items in ascending type order
(checksum before segmentation), so a driver can stop walking early.

## Frame items

| Item | Direction | Meaning |
|---|---|---|
| `S2XFRAME_CHECKSUM` (`S2XFrameChecksum`) | receive | the driver sets `S2XCS_IP_GOOD` / `IP_BAD` / `L4_GOOD` / `L4_BAD`; none set means not checked |
| | send | the stack sets `S2XCS_FILL_IP` and/or `S2XCS_FILL_L4`; for `FILL_L4` it puts the pseudo-header sum in the frame's checksum field and gives `s2xfc_Start` (where the sum starts) and `s2xfc_Offset` (where the result goes, from Start), byte offsets from the start of the Ethernet frame (virtio-net's model) |
| `S2XFRAME_SEGMENT` (`S2XFrameSegment`) | send | the frame holds the headers once and the whole payload; the card repeats the headers for every `s2xfs_SegSize` bytes of payload, adjusting lengths, sequence numbers, IP ids and checksums; `s2xfs_HdrLen` is Ethernet + IP + TCP |

A stack asks for a send-side item only when the driver's capability list
offers it. A driver that meets an item type it does not know skips it.

## Numbers

- Commands 0xC100-0xC102: in SANA-II's new-style range, clear of the
  official 0xC000-0xC008. The magic in `S2X_QUERY`'s answer tells these
  commands apart from any official command that might one day use the same
  number.
- Item types: capabilities from 0x0001 (types 1-31 have a bit in
  `s2xq_Features`), frame items from 0x1001. A type with `S2XITEM_PRIVATE`
  (bit 31) set belongs to one driver or stack and is ignored by others.

## Versions

Version 1 is this document. A later version may add commands, item types
and fields at the end of a structure; it never changes the meaning of what
is here. A driver reports the version it implements in `s2xq_Version`; a
stack uses only what both sides know.

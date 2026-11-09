# SANA-II extensions (unofficial), version 1

Commands, structures and item types that let a SANA-II network driver and a
protocol stack use what modern network cards offer:

- several frames per request (batches);
- checksums checked or computed by the card;
- segmentation of large TCP sends;
- several receive and transmit queues (receive-side scaling, RSS);
- settings: MTU (jumbo frames), interrupt moderation, flow control.

The C header is `include/devices/sana2ext.h`; the Rust module is
`amigaos4::sana2ext` (feature `sana2`). The first implementations are to be
the igb driver and rustsocket.

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
large send instead of dozens of frames; RSS lets one stack task per CPU take
its own share of the traffic; jumbo frames cut the frame count sixfold on a
LAN.

## Conventions

The design follows patterns already in the SDK:

- **Sized structures.** Every structure carries its size (`s2xq_Size`,
  `s2xf_Size`, `s2xb_Size`, `s2xc_Size`), as `struct Sana2Connection` and
  the SANA-II hook messages do. A reader uses only the fields that size
  covers; a newer version adds fields at the end.
- **Typed, sized items.** Capabilities, per-frame extras and settings are
  items: a common header (`struct S2XItem`: a `MinNode`, a type and the
  item's size) embedded as the first member of each item, linked in a
  `MinList`, as `expansion/pci.h`'s `struct PCICapability`. A reader walks
  the list, uses the types it knows and steps over the others
  (`S2X_CONFIGURE` is the exception: it refuses what it does not know).
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
  they are checked with the AmigaOS compiler (`tools/check-layout.sh`) and
  with the host's compilers against stand-in exec headers
  (`tools/check-sana2ext-host.sh`: x86-64, PowerPC, PowerPC64 so far), and
  the Rust module checks the same formulas on every target it builds for.
- **Types.** Explicit widths only (`uint32`, `uint16`, `uint8`, pointers).
  Sizes and lengths are 32-bit.
- **Byte order.** Every structure field is in the host's byte order. Frame
  data is as on the wire (network order). `S2X_MAGIC` is compared as an
  integer, never as bytes. A checksum's starting value that the stack puts
  into a frame (`S2XFRAME_CHECKSUM` on send) is in network order, as on the
  wire. An RSS key is a byte string, used as given.
- **Several CPUs.** Exec's message passing is the only hand-over between a
  driver and a stack: the driver writes everything in a batch before it
  replies and touches nothing after; the stack reads nothing before it has
  the reply (WaitIO / GetMsg). The capability list a driver returns never
  changes between OpenDevice and CloseDevice, so any task may read it.
  Requests for different queues may be sent from different tasks at the
  same time. A driver protects its own state with mutexes or semaphores
  (one per queue where it can), never Forbid/Disable, and reaches frame
  buffers only through the copy hooks and the DMA calls (StartDMA /
  GetDMAList / EndDMA), never by assuming caches are coherent.

## Discovery

1. `NSCMD_DEVICEQUERY`: a driver that has the extensions lists `S2X_QUERY`,
   and the other commands it supports, in `SupportedCommands`.
2. `S2X_QUERY` (0xC100): `ios2_StatData` points to a `struct S2XQuery`
   whose `s2xq_Size` the caller has set. The driver fills what fits:
   `s2xq_Magic` (`S2X_MAGIC`), `s2xq_Version` (the version it implements),
   `s2xq_Revision` (its own), `s2xq_Features` (one bit per capability, for a
   quick test) and `s2xq_Capabilities`, a pointer to its own read-only list
   of capability items.

A stack trusts the extensions only when the command succeeds and the magic
matches. Any other answer means plain SANA-II.

Capabilities are limits. The current settings are read with
`S2X_CONFIGURE` and `S2XCF_READ`.

| Capability | Item | Says |
|---|---|---|
| `S2XCAP_BATCH` | `S2XCapBatch` | the batch commands work; the most frames per batch |
| `S2XCAP_RXCHECKSUM` | `S2XCapChecksum` | which checksums the card checks on receive |
| `S2XCAP_TXCHECKSUM` | `S2XCapChecksum` | which checksums the card computes on send |
| `S2XCAP_SEGMENT` | `S2XCapSegment` | TCP segmentation: protocols, largest send, most segments, smallest segment size |
| `S2XCAP_MTU` | `S2XCapMTU` | the MTUs `S2XSET_MTU` accepts |
| `S2XCAP_MODERATION` | `S2XCapModeration` | the longest interrupt interval; whether receive and transmit can be set apart |
| `S2XCAP_FLOWCONTROL` | `S2XCapFlowControl` | which pause-frame modes the card has |
| `S2XCAP_RSS` | `S2XCapRSS` | hash types, most receive and transmit queues, key and indirection table sizes |

Every frame item rides on a batch, so a driver that offers receive or
transmit checksums, segmentation or RSS also offers `S2XCAP_BATCH`.
Segmentation also needs transmit checksums for the same protocols.

## Batches

`S2X_READBATCH` (0xC101) and `S2X_WRITEBATCH` (0xC102): `ios2_StatData`
points to a `struct S2XBatch`, a list of `struct S2XFrame`. Each frame
points to its own `IOSana2Req`, an opened copy of the batch's request (same
device, unit and buffer management), and carries a list of frame items (may
be empty). `s2xb_Queue` names the queue the batch is for: 0 without RSS.

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
and its items it knows (checksum results, the RSS hash). It sets
`s2xb_Done` to the number of frames filled, then replies the batch.

**Which request gets a frame.** A received frame goes to the oldest waiting
request for its packet type: a `CMD_READ`, or a `READBATCH` whose
`s2xb_Queue` is the queue the card put the frame on. `CMD_READ` takes frames
from any queue. A stack that wants every frame in batches posts no
`CMD_READ` for that type. This also holds across openers: while another
opener (a second stack, a monitoring tool) posts `CMD_READ` for a packet
type, frames of that type no longer keep per-queue order. That is a known
limit of version 1.

**WRITEBATCH.** Each frame's request is set as for `CMD_WRITE`. The driver
sends frames in list order on queue `s2xb_Queue` (plain `CMD_WRITE` uses
queue 0) and counts those it took in `s2xb_Done`. At the first frame it
refuses it stops: that frame's request has the error (`io_Error`,
`ios2_WireError`), and so does the batch's request. Frames on one queue go
out in order; frames on different queues have no order between them.

**Frames after the first `s2xb_Done`.** After a read batch their requests
and items are undefined. After a write batch they are untouched (not sent,
no error set), so the stack sends them again.

**Abort.** `AbortIO` is called on the batch's request only. A read batch
comes back with `IOERR_ABORTED` and whatever frames are filled (counted in
`s2xb_Done`); a write batch with the frames already taken counted.

**Allocation.** The stack allocates batches, frames and items once (shared
memory: `MEMF_SHARED`) and reuses them; the driver never allocates or frees
any of them. The stack keeps a frame's items in ascending type order
(checksum, segmentation, RSS), so a driver can stop walking early.

## Frame items

Offsets in frame items count from the first byte of the frame's data as the
buffer management copies it: the network-layer header (IP) for a normal
frame, the link header (Ethernet) with `SANA2IOF_RAW`.

| Item | Direction | Meaning |
|---|---|---|
| `S2XFRAME_CHECKSUM` (`S2XFrameChecksum`) | receive | the driver sets `S2XCS_IP_GOOD` / `IP_BAD` / `L4_GOOD` / `L4_BAD`; none set means not checked |
| | send | the stack sets `S2XCS_FILL_IP` and/or `S2XCS_FILL_L4`; for `FILL_L4` it puts the pseudo-header sum in the frame's checksum field and gives `s2xfc_Start` (where the sum starts) and `s2xfc_Offset` (where the result goes, from Start) (virtio-net's model) |
| `S2XFRAME_SEGMENT` (`S2XFrameSegment`) | send | see below |
| `S2XFRAME_RSS` (`S2XFrameRSS`) | receive | the card's Toeplitz hash and the hash type it used (0: not hashed), so the stack finds the connection without hashing again |

**Segmentation.** The frame's data holds the headers once and the whole
payload; `ios2_DataLength` may be up to `s2xcs_MaxSize`, more than the MTU.
The card repeats the headers for every `s2xfs_SegSize` bytes of payload,
adjusting lengths, sequence numbers, IP ids and checksums.
`s2xfs_HdrLen` is the header bytes in the data: IP + TCP, plus the link
header with `SANA2IOF_RAW`. The frame also carries an `S2XFRAME_CHECKSUM`
with `S2XCS_FILL_L4` (and `S2XCS_FILL_IP` for IPv4), whose pseudo-header
sum the stack computes with a length of 0: the card adds each segment's
length.

A stack asks for a send-side item only when the driver's capability list
offers it. A driver that meets a frame item type it does not know skips it.

## Settings: S2X_CONFIGURE

`S2X_CONFIGURE` (0xC103): `ios2_StatData` points to a
`struct S2XConfigure` holding a list of setting items.

| Setting | Item | Values |
|---|---|---|
| `S2XSET_MTU` | `S2XSetMTU` | the MTU (packet data, without the link header, as `Sana2DeviceQuery`'s MTU), within `S2XCapMTU`; `S2_DEVICEQUERY` reports it afterwards |
| `S2XSET_MODERATION` | `S2XSetModeration` | microseconds between interrupts for receive and transmit, 0 is off; without `S2XMOD_TX` the receive value applies to both |
| `S2XSET_FLOWCONTROL` | `S2XSetFlowControl` | the `S2XFC_` modes to use, 0 is off |
| `S2XSET_RSS` | `S2XSetRSS` | hash types, number of receive queues (1 turns RSS off), the key (exactly `s2xcr_KeySize` bytes) and the indirection table (exactly `s2xcr_TableSize` entries); transmit queues are the smaller of the receive queues and `s2xcr_MaxTxQueues` |

- **Changing.** Without `S2XCF_READ` the driver checks every item before it
  changes anything. If it refuses one (a type it does not know, a value
  outside its capability, or a change it cannot make while other openers
  are open, `S2ERR_BAD_STATE`) it changes nothing, points `s2xc_Failed` at
  that item and returns the error (`S2ERR_NOT_SUPPORTED`,
  `S2ERR_BAD_ARGUMENT` or `S2ERR_BAD_STATE`). Otherwise it applies the items
  in list order and writes back the values it applied, since a value may be
  rounded (moderation to the card's step, for example).
- **Settings and posted requests.** A setting never changes under a
  request in flight. The driver answers `S2ERR_BAD_STATE` (with
  `s2xc_Failed` set) to `S2XSET_MTU` while any opener has a receive request
  posted (`CMD_READ` or `S2X_READBATCH`), since those buffers were sized
  for the old MTU, and to `S2XSET_RSS` while requests are posted on a queue
  it would remove. A stack therefore changes the MTU with no receive request
  posted (it aborts and waits for its own first), then reads the new MTU
  with `S2_DEVICEQUERY` and sizes its buffers from it.
- **Reading.** With `S2XCF_READ` the driver fills every item with the
  current setting and changes nothing.
- **Scope.** Settings belong to the unit: a change affects every opener.
  After a change the driver sends `S2EVENT_CONFIGCHANGED`. A change that
  needs the link reset (often the MTU) may also bring
  `S2EVENT_OFFLINE` / `S2EVENT_ONLINE`.
- **Memory.** The items and any buffers they point to (the RSS key and
  table) are the caller's; the driver uses them only until it replies.

## RSS: several queues

With `S2XCAP_RSS` the card hashes each received frame (Toeplitz, as in
Microsoft's RSS specification and virtio-net's RSS), looks the hash's low
bits up in an indirection table of `s2xcr_TableSize` entries, and puts the
frame on the queue that entry names. The stack:

1. sets the queues with `S2XSET_RSS` (number of queues, hash types; a key
   and table, or NULL for the driver's own key and an even spread);
2. posts `S2X_READBATCH` requests for each queue (`s2xb_Queue`), typically
   from one task per CPU, each with its own reply port;
3. sends on the queue of the task that owns the connection
   (`S2X_WRITEBATCH` with `s2xb_Queue`), so tasks do not contend on one
   transmit queue.

Frames of one connection always hash to the same queue, so each task sees
its connections' frames in order. Changing the table may move a
connection to another queue; the stack does so only when it can hand the
connection over. With one queue, or without RSS, everything is queue 0.

## Numbers

- Commands 0xC100-0xC103: in SANA-II's new-style range, clear of the
  official 0xC000-0xC008. The magic in `S2X_QUERY`'s answer tells these
  commands apart from any official command that might one day use the same
  number.
- Item types: capabilities from 0x0001 (types 1-31 have a bit in
  `s2xq_Features`), frame items from 0x1001, settings from 0x2001. A type
  with `S2XITEM_PRIVATE` (bit 31) set belongs to one driver or stack and is
  ignored by others (refused by `S2X_CONFIGURE`).

## Versions

Version 1 is this document. A later version may add commands, item types
and fields at the end of a structure; it never changes the meaning of what
is here. A driver reports the version it implements in `s2xq_Version`; a
stack uses only what both sides know.

Possible later items, which fit without changing anything here: hardware
receive coalescing (a frame item with the merged length and segment
count), VLAN tag insertion and stripping, and hardware timestamps.

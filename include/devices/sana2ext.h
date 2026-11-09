#ifndef DEVICES_SANA2EXT_H
#define DEVICES_SANA2EXT_H

/*
** sana2ext.h -- SANA-II extensions (unofficial), version 1.
**
** Commands, structures and item types that let a SANA-II device driver and
** a protocol stack use what modern network hardware offers: several frames
** per request, checksums computed or checked by the card, segmentation of
** large sends, several receive and transmit queues (RSS), and settings
** (MTU, interrupt moderation, flow control). They are NOT part of SANA-II
** Revision 7 or of the AmigaOS 4 SDK. A driver that does not implement
** them answers IOERR_NOCMD, and the stack then uses plain SANA-II.
**
** The full rules (ownership, completion, abort, versioning, portability)
** are in docs/devices/sana2ext.md of the amigaos4 crate.
**
** Conventions:
** - Every structure carries its size, and items are typed, sized and
**   linked through a MinNode (as expansion/pci.h's struct PCICapability),
**   so a reader skips what it does not know.
** - Portable: every field lies at an offset that is a multiple of its own
**   size, and every structure is a multiple of the pointer size long, so
**   the layout is the same with the SDK's pack(2) and with natural
**   alignment, with 32- and 64-bit pointers. All fields are in host byte
**   order; frame data is as on the wire.
**
** Released under the MIT licence, as the amigaos4 crate.
*/

#ifndef EXEC_TYPES_H
#include <exec/types.h>
#endif
#ifndef EXEC_NODES_H
#include <exec/nodes.h>
#endif
#ifndef EXEC_LISTS_H
#include <exec/lists.h>
#endif
#ifndef SANA2_SANA2DEVICE_H
#include <devices/sana2.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif

#ifdef __GNUC__
   #ifdef __PPC__
    #pragma pack(2)
   #endif
#elif defined(__VBCC__)
   #pragma amiga-align
#endif

/****************************************************************************/

/* Answered by S2X_QUERY in s2xq_Magic: 'S2XT' as a number (compare it as
** an integer, never as bytes). */
#define S2X_MAGIC    0x53325854UL
/* This header's version. A new version may add commands, item types and
** fields at the end of a structure; it does not change what is here. */
#define S2X_VERSION  1

/****************************************************************************/

/*
** Commands. ios2_StatData points to the command's structure (as for
** S2_GETGLOBALSTATS); the request is an opened copy (same device, unit and
** buffer management). Listed in NSCMD_DEVICEQUERY's SupportedCommands by a
** driver that has them.
*/
#define S2X_QUERY       0xC100  /* struct S2XQuery: what the driver offers */
#define S2X_READBATCH   0xC101  /* struct S2XBatch: receive several frames */
#define S2X_WRITEBATCH  0xC102  /* struct S2XBatch: send several frames */
#define S2X_CONFIGURE   0xC103  /* struct S2XConfigure: read or change settings */

/****************************************************************************/

/*
** Every item starts with this header. Items are linked in a MinList; a
** reader walks the list, uses the types it knows and steps over the
** others (except S2X_CONFIGURE, which refuses a type it does not know).
** s2xi_Size is the whole item, header included: a reader uses only the
** fields that size covers.
*/
struct S2XItem
{
    struct MinNode s2xi_Node;   /* links items */
    uint32         s2xi_Type;   /* S2XCAP_..., S2XFRAME_... or S2XSET_... */
    uint32         s2xi_Size;   /* bytes of the whole item */
};

/* Item types: capabilities (S2X_QUERY), frame items (S2XFrame), settings
** (S2X_CONFIGURE). A type with S2XITEM_PRIVATE set belongs to one driver
** or stack; others ignore it. */
#define S2XCAP_BATCH        0x0001  /* struct S2XCapBatch */
#define S2XCAP_RXCHECKSUM   0x0002  /* struct S2XCapChecksum */
#define S2XCAP_TXCHECKSUM   0x0003  /* struct S2XCapChecksum */
#define S2XCAP_SEGMENT      0x0004  /* struct S2XCapSegment */
#define S2XCAP_MTU          0x0005  /* struct S2XCapMTU */
#define S2XCAP_MODERATION   0x0006  /* struct S2XCapModeration */
#define S2XCAP_FLOWCONTROL  0x0007  /* struct S2XCapFlowControl */
#define S2XCAP_RSS          0x0008  /* struct S2XCapRSS */

#define S2XFRAME_CHECKSUM   0x1001  /* struct S2XFrameChecksum */
#define S2XFRAME_SEGMENT    0x1002  /* struct S2XFrameSegment */
#define S2XFRAME_RSS        0x1003  /* struct S2XFrameRSS */

#define S2XSET_MTU          0x2001  /* struct S2XSetMTU */
#define S2XSET_MODERATION   0x2002  /* struct S2XSetModeration */
#define S2XSET_FLOWCONTROL  0x2003  /* struct S2XSetFlowControl */
#define S2XSET_RSS          0x2004  /* struct S2XSetRSS */

#define S2XITEM_PRIVATE     0x80000000UL

/****************************************************************************/

/*
** S2X_QUERY. The caller sets s2xq_Size (the bytes it provides); the
** driver fills what fits. s2xq_Capabilities is the driver's own list of
** S2XCap... items: read only, unchanged from OpenDevice to CloseDevice, so
** any task on any CPU may read it while the opener is open. Capabilities
** are limits; the current settings are read with S2X_CONFIGURE.
*/
struct S2XQuery
{
    uint32                  s2xq_Size;          /* bytes the caller provides */
    uint32                  s2xq_Magic;         /* S2X_MAGIC */
    uint16                  s2xq_Version;       /* S2X_VERSION the driver implements */
    uint16                  s2xq_Revision;      /* the driver's own revision */
    uint32                  s2xq_Features;      /* S2XF_... bits: a quick test */
    const struct MinList *  s2xq_Capabilities;  /* S2XCap... items */
};

/* s2xq_Features: one bit per capability item present (capability types
** 1..31 only). Every frame item rides on a batch, so a driver that offers
** RXCHECKSUM, TXCHECKSUM, SEGMENT or RSS also offers BATCH. */
#define S2XF_BATCH          (1UL << S2XCAP_BATCH)
#define S2XF_RXCHECKSUM     (1UL << S2XCAP_RXCHECKSUM)
#define S2XF_TXCHECKSUM     (1UL << S2XCAP_TXCHECKSUM)
#define S2XF_SEGMENT        (1UL << S2XCAP_SEGMENT)
#define S2XF_MTU            (1UL << S2XCAP_MTU)
#define S2XF_MODERATION     (1UL << S2XCAP_MODERATION)
#define S2XF_FLOWCONTROL    (1UL << S2XCAP_FLOWCONTROL)
#define S2XF_RSS            (1UL << S2XCAP_RSS)

/* S2XCAP_BATCH: S2X_READBATCH and S2X_WRITEBATCH. */
struct S2XCapBatch
{
    struct S2XItem s2xcb_Header;
    uint32         s2xcb_MaxFrames;   /* frames one batch may carry */
    uint32         s2xcb_Flags;       /* none defined: 0 */
};

/* S2XCAP_RXCHECKSUM / S2XCAP_TXCHECKSUM: which checksums the card checks
** on receive / computes on send. */
struct S2XCapChecksum
{
    struct S2XItem s2xcc_Header;
    uint32         s2xcc_Protocols;   /* S2XCSUM_... bits */
    uint32         s2xcc_Reserved;    /* 0 */
};

#define S2XCSUM_IPV4        0x0001  /* the IPv4 header checksum */
#define S2XCSUM_TCPV4       0x0002
#define S2XCSUM_UDPV4       0x0004
#define S2XCSUM_TCPV6       0x0008
#define S2XCSUM_UDPV6       0x0010

/* S2XCAP_SEGMENT: the card cuts one large TCP send into frames. Needs
** S2XCAP_TXCHECKSUM for the same protocols. */
struct S2XCapSegment
{
    struct S2XItem s2xcs_Header;
    uint32         s2xcs_Protocols;   /* S2XCSUM_TCPV4 / S2XCSUM_TCPV6 */
    uint32         s2xcs_MaxSize;     /* largest ios2_DataLength of one send */
    uint16         s2xcs_MaxSegments; /* most frames one send may become */
    uint16         s2xcs_MinSegSize;  /* smallest segment size accepted */
    uint32         s2xcs_Reserved;    /* 0 */
};

/* S2XCAP_MTU: the MTUs S2XSET_MTU accepts (MTU as in Sana2DeviceQuery:
** packet data, without the link header). */
struct S2XCapMTU
{
    struct S2XItem s2xcm_Header;
    uint32         s2xcm_MinMTU;
    uint32         s2xcm_MaxMTU;
};

/* S2XCAP_MODERATION: interrupt moderation (the shortest time between two
** interrupts) S2XSET_MODERATION accepts. */
struct S2XCapModeration
{
    struct S2XItem s2xco_Header;
    uint32         s2xco_MaxMicros;   /* longest interval; 0 is always allowed (off) */
    uint32         s2xco_Flags;       /* S2XMOD_...: which directions can be set */
};

#define S2XMOD_RX           0x0001  /* receive interval settable */
#define S2XMOD_TX           0x0002  /* transmit interval settable apart from receive */

/* S2XCAP_FLOWCONTROL: Ethernet pause frames (IEEE 802.3x). */
struct S2XCapFlowControl
{
    struct S2XItem s2xcf_Header;
    uint32         s2xcf_Modes;       /* S2XFC_... the card supports */
    uint32         s2xcf_Reserved;    /* 0 */
};

#define S2XFC_RX            0x0001  /* obey pause frames from the link partner */
#define S2XFC_TX            0x0002  /* send pause frames when receive is full */
#define S2XFC_AUTO          0x0004  /* agree RX/TX through autonegotiation */

/* S2XCAP_RSS: several receive queues (receive-side scaling) and transmit
** queues. The card hashes each received frame (Toeplitz, as Microsoft's
** RSS specification and virtio-net's RSS), looks the hash's low bits up in
** an indirection table of TableSize entries and puts the frame on that
** queue. Each queue has its own S2X_READBATCH requests, so one stack task
** per CPU can take its own queue. Per-queue order holds only while no
** opener posts CMD_READ for that packet type (CMD_READ takes any queue). */
struct S2XCapRSS
{
    struct S2XItem s2xcr_Header;
    uint32         s2xcr_HashTypes;   /* S2XRSS_... the card can hash */
    uint16         s2xcr_MaxQueues;   /* receive queues */
    uint16         s2xcr_KeySize;     /* bytes of the hash key */
    uint16         s2xcr_TableSize;   /* entries of the indirection table */
    uint16         s2xcr_MaxTxQueues; /* transmit queues */
    uint32         s2xcr_Reserved;    /* 0 */
};

#define S2XRSS_IPV4         0x0001  /* addresses of IPv4 frames */
#define S2XRSS_TCPV4        0x0002  /* addresses and ports of IPv4 TCP */
#define S2XRSS_UDPV4        0x0004
#define S2XRSS_IPV6         0x0008
#define S2XRSS_TCPV6        0x0010
#define S2XRSS_UDPV6        0x0020

/****************************************************************************/

/*
** One frame of a batch: the standard request that describes it, and its
** items. s2xf_Req is an opened copy (same device, unit and buffer
** management as the batch's request); its fields mean what they mean for
** CMD_READ / CMD_WRITE (ios2_PacketType, addresses, ios2_DataLength,
** ios2_Data: the buffer management cookie, io_Flags, io_Error,
** ios2_WireError). It is never sent on its own while in a batch.
**
** Offsets in frame items count from the first byte of the frame's data as
** the buffer management copies it: the network-layer header (IP) for a
** normal frame, the link header (Ethernet) with SANA2IOF_RAW.
*/
struct S2XFrame
{
    struct MinNode       s2xf_Node;      /* links frames in a batch */
    struct IOSana2Req *  s2xf_Req;       /* the frame's standard request */
    struct MinList       s2xf_Items;     /* S2XFrame... items, may be empty */
    uint32               s2xf_Size;      /* bytes of this structure */
    uint32               s2xf_Reserved;  /* 0 */
};

/* S2XFRAME_CHECKSUM. On receive the driver sets the result flags; on send
** the stack asks for checksums (virtio-net's model: the stack puts the
** pseudo-header sum in the frame's checksum field, in network byte order
** as on the wire; the card sums from s2xfc_Start to the end of the frame
** and stores the result at s2xfc_Start + s2xfc_Offset). */
struct S2XFrameChecksum
{
    struct S2XItem s2xfc_Header;
    uint32         s2xfc_Flags;       /* S2XCS_... */
    uint16         s2xfc_Start;       /* send: where the sum starts */
    uint16         s2xfc_Offset;      /* send: where it goes, from Start */
};

/* Receive results (driver sets them; none set: not checked). */
#define S2XCS_IP_GOOD       0x0001  /* IPv4 header checksum right */
#define S2XCS_IP_BAD        0x0002
#define S2XCS_L4_GOOD       0x0004  /* TCP/UDP checksum right */
#define S2XCS_L4_BAD        0x0008
/* Send requests (stack sets them, only for protocols in S2XCAP_TXCHECKSUM). */
#define S2XCS_FILL_IP       0x0100  /* compute the IPv4 header checksum */
#define S2XCS_FILL_L4       0x0200  /* compute the sum described by Start/Offset */

/* S2XFRAME_SEGMENT: send only, needs S2XCAP_SEGMENT. ios2_Data holds the
** headers once and the whole payload, up to s2xcs_MaxSize bytes (more than
** the MTU); the card repeats the headers for every segment (adjusting
** lengths, sequence numbers, IP ids and checksums). The frame also carries
** an S2XFRAME_CHECKSUM with S2XCS_FILL_L4 (and S2XCS_FILL_IP for IPv4),
** whose pseudo-header sum is computed with a length of 0. */
struct S2XFrameSegment
{
    struct S2XItem s2xfs_Header;
    uint32         s2xfs_Protocol;    /* S2XCSUM_TCPV4 or S2XCSUM_TCPV6 */
    uint16         s2xfs_SegSize;     /* payload bytes per segment (the MSS) */
    uint16         s2xfs_HdrLen;      /* header bytes in the data: IP + TCP (+ link with RAW) */
};

/* S2XFRAME_RSS: receive only, needs S2XCAP_RSS. The card's hash, so the
** stack finds the connection without hashing again. */
struct S2XFrameRSS
{
    struct S2XItem s2xfr_Header;
    uint32         s2xfr_Hash;        /* the Toeplitz hash */
    uint32         s2xfr_HashType;    /* the one S2XRSS_... used; 0: not hashed */
};

/****************************************************************************/

/*
** S2X_READBATCH / S2X_WRITEBATCH. While the batch's request is in flight
** the driver owns this structure, its frames, their requests and items;
** it writes everything before it replies and nothing after, and the caller
** reads nothing before it has the reply (exec's message passing is the
** only hand-over, also between CPUs).
**
** READBATCH: frames are filled in list order, as many as are waiting, at
** least one; the request completes like CMD_READ (it waits for the first
** frame). The batch request's ios2_PacketType and io_Flags (SANA2IOF_RAW)
** apply to every frame. A received frame goes to the oldest waiting request
** for its packet type: a CMD_READ, or a READBATCH for the frame's queue.
** WRITEBATCH: frames are sent in list order; s2xb_Done counts those the
** driver took. At the first one it refuses it stops: that frame's request
** has the error, and so does the batch's.
** Frames after the first s2xb_Done: undefined after a read batch; untouched
** (not sent, no error set) after a write batch, so the stack sends them
** again.
*/
struct S2XBatch
{
    uint32         s2xb_Size;    /* bytes of this structure */
    uint32         s2xb_Count;   /* frames in the list */
    struct MinList s2xb_Frames;  /* S2XFrame nodes */
    uint32         s2xb_Done;    /* set by the driver */
    uint32         s2xb_Queue;   /* receive / transmit queue; 0 without RSS */
};

/****************************************************************************/

/*
** S2X_CONFIGURE: read or change the unit's settings, given as a list of
** S2XSet... items. Settings belong to the unit: a change affects every
** opener, and the driver sends S2EVENT_CONFIGCHANGED after one.
**
** Without S2XCF_READ the driver checks every item before it changes
** anything: on an item it refuses (an unknown type, a value outside its
** capability, or S2ERR_BAD_STATE for a change it cannot make now) it
** changes nothing, points s2xc_Failed at that item and returns the error.
** S2ERR_BAD_STATE is the answer while any opener has receive requests
** posted (CMD_READ or READBATCH) when the item is S2XSET_MTU, or has
** requests posted on a queue that S2XSET_RSS would remove. Otherwise it applies them in list order and
** writes back the values it applied (a value may be rounded).
** With S2XCF_READ it fills every item with the current setting.
** The items and any buffers they point to are the caller's; the driver
** uses them only until it replies.
*/
struct S2XConfigure
{
    uint32           s2xc_Size;    /* bytes of this structure */
    uint32           s2xc_Flags;   /* S2XCF_... */
    struct MinList   s2xc_Items;   /* S2XSet... items */
    struct S2XItem * s2xc_Failed;  /* set by the driver on a refusal, else NULL */
};

#define S2XCF_READ          0x0001  /* fill the items with the current settings */

/* S2XSET_MTU: the unit's MTU; S2_DEVICEQUERY then reports it. A stack
** changes it with no receive request posted, then sizes its receive
** buffers from S2_DEVICEQUERY. */
struct S2XSetMTU
{
    struct S2XItem s2xsm_Header;
    uint32         s2xsm_MTU;
    uint32         s2xsm_Reserved;    /* 0 */
};

/* S2XSET_MODERATION: microseconds between interrupts; 0 is off. Without
** S2XMOD_TX the receive value applies to both. */
struct S2XSetModeration
{
    struct S2XItem s2xso_Header;
    uint32         s2xso_RxMicros;
    uint32         s2xso_TxMicros;
};

/* S2XSET_FLOWCONTROL: the S2XFC_... modes to use; 0 is off. */
struct S2XSetFlowControl
{
    struct S2XItem s2xsf_Header;
    uint32         s2xsf_Modes;
    uint32         s2xsf_Reserved;    /* 0 */
};

/* S2XSET_RSS: s2xsr_Queues receive queues; 1 turns RSS off (everything
** on queue 0). Transmit queues: the smaller of s2xsr_Queues and
** s2xcr_MaxTxQueues. s2xsr_Key is exactly s2xcr_KeySize bytes and
** s2xsr_Table exactly s2xcr_TableSize queue numbers (s2xsr_TableSize must
** equal it); either may be NULL: the driver keeps its own key and spreads
** the table evenly over the queues. With S2XCF_READ, non-NULL Key and
** Table are filled. */
struct S2XSetRSS
{
    struct S2XItem s2xsr_Header;
    uint32         s2xsr_HashTypes;   /* S2XRSS_... to hash */
    uint16         s2xsr_Queues;      /* receive queues */
    uint16         s2xsr_TableSize;   /* entries in s2xsr_Table: s2xcr_TableSize */
    uint8 *        s2xsr_Key;
    uint8 *        s2xsr_Table;       /* queue number per entry */
};

/****************************************************************************/

/* The sizes, as formulas of the pointer size, on every CPU. */
#if defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
#define S2X_P_ sizeof(APTR)
#define S2X_H_ (2 * S2X_P_ + 8)
_Static_assert(sizeof(struct S2XItem) == S2X_H_, "S2XItem");
_Static_assert(sizeof(struct S2XQuery) == 16 + S2X_P_, "S2XQuery");
_Static_assert(sizeof(struct S2XCapBatch) == S2X_H_ + 8, "S2XCapBatch");
_Static_assert(sizeof(struct S2XCapChecksum) == S2X_H_ + 8, "S2XCapChecksum");
_Static_assert(sizeof(struct S2XCapSegment) == S2X_H_ + 16, "S2XCapSegment");
_Static_assert(sizeof(struct S2XCapMTU) == S2X_H_ + 8, "S2XCapMTU");
_Static_assert(sizeof(struct S2XCapModeration) == S2X_H_ + 8, "S2XCapModeration");
_Static_assert(sizeof(struct S2XCapFlowControl) == S2X_H_ + 8, "S2XCapFlowControl");
_Static_assert(sizeof(struct S2XCapRSS) == S2X_H_ + 16, "S2XCapRSS");
_Static_assert(sizeof(struct S2XFrame) == 6 * S2X_P_ + 8, "S2XFrame");
_Static_assert(sizeof(struct S2XFrameChecksum) == S2X_H_ + 8, "S2XFrameChecksum");
_Static_assert(sizeof(struct S2XFrameSegment) == S2X_H_ + 8, "S2XFrameSegment");
_Static_assert(sizeof(struct S2XFrameRSS) == S2X_H_ + 8, "S2XFrameRSS");
_Static_assert(sizeof(struct S2XBatch) == 16 + 3 * S2X_P_, "S2XBatch");
_Static_assert(sizeof(struct S2XConfigure) == 8 + 4 * S2X_P_, "S2XConfigure");
_Static_assert(sizeof(struct S2XSetMTU) == S2X_H_ + 8, "S2XSetMTU");
_Static_assert(sizeof(struct S2XSetModeration) == S2X_H_ + 8, "S2XSetModeration");
_Static_assert(sizeof(struct S2XSetFlowControl) == S2X_H_ + 8, "S2XSetFlowControl");
_Static_assert(sizeof(struct S2XSetRSS) == S2X_H_ + 8 + 2 * S2X_P_, "S2XSetRSS");
#undef S2X_H_
#undef S2X_P_
#endif

#ifdef __GNUC__
   #ifdef __PPC__
    #pragma pack()
   #endif
#elif defined(__VBCC__)
   #pragma default-align
#endif

#ifdef __cplusplus
}
#endif

#endif /* DEVICES_SANA2EXT_H */

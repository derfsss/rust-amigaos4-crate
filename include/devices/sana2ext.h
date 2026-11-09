#ifndef DEVICES_SANA2EXT_H
#define DEVICES_SANA2EXT_H

/*
** sana2ext.h -- SANA-II extensions (unofficial), version 1.
**
** Commands, structures and item types that let a SANA-II device driver and
** a protocol stack use what modern network hardware offers: several frames
** per request, checksums computed or checked by the card, segmentation of
** large sends. They are NOT part of SANA-II Revision 7 or of the AmigaOS 4
** SDK. A driver that does not implement them answers IOERR_NOCMD, and the
** stack then uses plain SANA-II.
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
**   order;
**   frame data is as on the wire.
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

/****************************************************************************/

/*
** Every item starts with this header. Items are linked in a MinList; a
** reader walks the list, uses the types it knows and steps over the
** others. s2xi_Size is the whole item, header included: a reader uses only
** the fields that size covers.
*/
struct S2XItem
{
    struct MinNode s2xi_Node;   /* links items */
    uint32         s2xi_Type;   /* S2XCAP_... or S2XFRAME_... */
    uint32         s2xi_Size;   /* bytes of the whole item */
};

/* Item types: capabilities (S2X_QUERY), frame items (S2XFrame). A type with
** S2XITEM_PRIVATE set belongs to one driver or stack; others ignore it. */
#define S2XCAP_BATCH        0x0001  /* struct S2XCapBatch */
#define S2XCAP_RXCHECKSUM   0x0002  /* struct S2XCapChecksum */
#define S2XCAP_TXCHECKSUM   0x0003  /* struct S2XCapChecksum */
#define S2XCAP_SEGMENT      0x0004  /* struct S2XCapSegment */

#define S2XFRAME_CHECKSUM   0x1001  /* struct S2XFrameChecksum */
#define S2XFRAME_SEGMENT    0x1002  /* struct S2XFrameSegment */

#define S2XITEM_PRIVATE     0x80000000UL

/****************************************************************************/

/*
** S2X_QUERY. The caller sets s2xq_Size (the bytes it provides); the
** driver fills what fits. s2xq_Capabilities is the driver's own list of
** S2XCap... items: read only, unchanged from OpenDevice to CloseDevice, so
** any task on any CPU may read it while the opener is open.
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
** 1..31 only). */
#define S2XF_BATCH          (1UL << S2XCAP_BATCH)
#define S2XF_RXCHECKSUM     (1UL << S2XCAP_RXCHECKSUM)
#define S2XF_TXCHECKSUM     (1UL << S2XCAP_TXCHECKSUM)
#define S2XF_SEGMENT        (1UL << S2XCAP_SEGMENT)

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

/* S2XCAP_SEGMENT: the card cuts one large TCP send into frames. */
struct S2XCapSegment
{
    struct S2XItem s2xcs_Header;
    uint32         s2xcs_Protocols;   /* S2XCSUM_TCPV4 / S2XCSUM_TCPV6 */
    uint32         s2xcs_MaxSize;     /* largest send (headers included) */
    uint16         s2xcs_MaxSegments; /* most frames one send may become */
    uint16         s2xcs_MinSegSize;  /* smallest segment size accepted */
    uint32         s2xcs_Reserved;    /* 0 */
};

/****************************************************************************/

/*
** One frame of a batch: the standard request that describes it, and its
** items. s2xf_Req is an opened copy (same device, unit and buffer
** management as the batch's request); its fields mean what they mean for
** CMD_READ / CMD_WRITE (ios2_PacketType, addresses, ios2_DataLength,
** ios2_Data: the buffer management cookie, io_Flags, io_Error,
** ios2_WireError). It is never sent on its own while in a batch.
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
** and stores the result at s2xfc_Start + s2xfc_Offset). Offsets count in
** bytes from the start of the Ethernet frame. */
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
** headers once and the whole payload; the card repeats the headers for
** every segment (adjusting lengths, sequence numbers, IP ids, checksums). */
struct S2XFrameSegment
{
    struct S2XItem s2xfs_Header;
    uint32         s2xfs_Protocol;    /* S2XCSUM_TCPV4 or S2XCSUM_TCPV6 */
    uint16         s2xfs_SegSize;     /* payload bytes per segment (the MSS) */
    uint16         s2xfs_HdrLen;      /* Ethernet + IP + TCP header bytes */
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
** apply to every frame.
** WRITEBATCH: frames are sent in list order; s2xb_Done counts those the
** driver took. At the first one it refuses it stops: that frame's request
** has the error, and so does the batch's.
*/
struct S2XBatch
{
    uint32         s2xb_Size;    /* bytes of this structure */
    uint32         s2xb_Count;   /* frames in the list */
    struct MinList s2xb_Frames;  /* S2XFrame nodes */
    uint32         s2xb_Done;    /* set by the driver */
    uint32         s2xb_Reserved; /* 0 */
};

/****************************************************************************/

/* The sizes, as formulas of the pointer size, on every CPU. */
#if defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
#define S2X_P_ sizeof(APTR)
_Static_assert(sizeof(struct S2XItem) == 2 * S2X_P_ + 8, "S2XItem");
_Static_assert(sizeof(struct S2XQuery) == 16 + S2X_P_, "S2XQuery");
_Static_assert(sizeof(struct S2XCapBatch) == sizeof(struct S2XItem) + 8, "S2XCapBatch");
_Static_assert(sizeof(struct S2XCapChecksum) == sizeof(struct S2XItem) + 8, "S2XCapChecksum");
_Static_assert(sizeof(struct S2XCapSegment) == sizeof(struct S2XItem) + 16, "S2XCapSegment");
_Static_assert(sizeof(struct S2XFrame) == 6 * S2X_P_ + 8, "S2XFrame");
_Static_assert(sizeof(struct S2XFrameChecksum) == sizeof(struct S2XItem) + 8, "S2XFrameChecksum");
_Static_assert(sizeof(struct S2XFrameSegment) == sizeof(struct S2XItem) + 8, "S2XFrameSegment");
_Static_assert(sizeof(struct S2XBatch) == 16 + 3 * S2X_P_, "S2XBatch");
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

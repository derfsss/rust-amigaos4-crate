#include <devices/sana2ext.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
/* sana2ext.rs (the header is the crate's own: include/devices/sana2ext.h).
** Written in the pointer size P: the same rule holds on 32- and 64-bit. */
#define P sizeof(APTR)
#define H (2 * P + 8)
A(sizeof(struct MinNode) == 2 * P && sizeof(struct MinList) == 3 * P);
A(sizeof(struct S2XItem) == H && offsetof(struct S2XItem, s2xi_Type) == 2 * P && offsetof(struct S2XItem, s2xi_Size) == 2 * P + 4);
A(sizeof(struct S2XQuery) == 16 + P && offsetof(struct S2XQuery, s2xq_Magic) == 4 && offsetof(struct S2XQuery, s2xq_Version) == 8);
A(offsetof(struct S2XQuery, s2xq_Revision) == 10 && offsetof(struct S2XQuery, s2xq_Features) == 12 && offsetof(struct S2XQuery, s2xq_Capabilities) == 16);
A(sizeof(struct S2XCapBatch) == H + 8 && offsetof(struct S2XCapBatch, s2xcb_MaxFrames) == H && offsetof(struct S2XCapBatch, s2xcb_Flags) == H + 4);
A(sizeof(struct S2XCapChecksum) == H + 8 && offsetof(struct S2XCapChecksum, s2xcc_Protocols) == H);
A(sizeof(struct S2XCapSegment) == H + 16 && offsetof(struct S2XCapSegment, s2xcs_MaxSize) == H + 4);
A(offsetof(struct S2XCapSegment, s2xcs_MaxSegments) == H + 8 && offsetof(struct S2XCapSegment, s2xcs_MinSegSize) == H + 10);
A(sizeof(struct S2XFrame) == 6 * P + 8 && offsetof(struct S2XFrame, s2xf_Req) == 2 * P && offsetof(struct S2XFrame, s2xf_Items) == 3 * P);
A(offsetof(struct S2XFrame, s2xf_Size) == 6 * P && offsetof(struct S2XFrame, s2xf_Reserved) == 6 * P + 4);
A(sizeof(struct S2XFrameChecksum) == H + 8 && offsetof(struct S2XFrameChecksum, s2xfc_Flags) == H);
A(offsetof(struct S2XFrameChecksum, s2xfc_Start) == H + 4 && offsetof(struct S2XFrameChecksum, s2xfc_Offset) == H + 6);
A(sizeof(struct S2XFrameSegment) == H + 8 && offsetof(struct S2XFrameSegment, s2xfs_Protocol) == H);
A(offsetof(struct S2XFrameSegment, s2xfs_SegSize) == H + 4 && offsetof(struct S2XFrameSegment, s2xfs_HdrLen) == H + 6);
A(sizeof(struct S2XBatch) == 16 + 3 * P && offsetof(struct S2XBatch, s2xb_Count) == 4 && offsetof(struct S2XBatch, s2xb_Frames) == 8);
A(offsetof(struct S2XBatch, s2xb_Done) == 8 + 3 * P && offsetof(struct S2XBatch, s2xb_Reserved) == 12 + 3 * P);
/* Not on an official SANA-II command */
A(S2X_QUERY > S2_SANA2HOOK && S2X_WRITEBATCH < 0xC200);

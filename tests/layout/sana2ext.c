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
A(offsetof(struct S2XBatch, s2xb_Done) == 8 + 3 * P && offsetof(struct S2XBatch, s2xb_Queue) == 12 + 3 * P);
A(sizeof(struct S2XCapMTU) == H + 8 && offsetof(struct S2XCapMTU, s2xcm_MinMTU) == H && offsetof(struct S2XCapMTU, s2xcm_MaxMTU) == H + 4);
A(sizeof(struct S2XCapModeration) == H + 8 && offsetof(struct S2XCapModeration, s2xco_Flags) == H + 4);
A(sizeof(struct S2XCapFlowControl) == H + 8 && offsetof(struct S2XCapFlowControl, s2xcf_Modes) == H);
A(sizeof(struct S2XCapRSS) == H + 16 && offsetof(struct S2XCapRSS, s2xcr_MaxQueues) == H + 4 && offsetof(struct S2XCapRSS, s2xcr_KeySize) == H + 6);
A(offsetof(struct S2XCapRSS, s2xcr_TableSize) == H + 8 && offsetof(struct S2XCapRSS, s2xcr_MaxTxQueues) == H + 10 && offsetof(struct S2XCapRSS, s2xcr_Reserved) == H + 12);
A(sizeof(struct S2XFrameRSS) == H + 8 && offsetof(struct S2XFrameRSS, s2xfr_Hash) == H && offsetof(struct S2XFrameRSS, s2xfr_HashType) == H + 4);
A(sizeof(struct S2XConfigure) == 8 + 4 * P && offsetof(struct S2XConfigure, s2xc_Flags) == 4);
A(offsetof(struct S2XConfigure, s2xc_Items) == 8 && offsetof(struct S2XConfigure, s2xc_Failed) == 8 + 3 * P);
A(sizeof(struct S2XSetMTU) == H + 8 && offsetof(struct S2XSetMTU, s2xsm_MTU) == H);
A(sizeof(struct S2XSetModeration) == H + 8 && offsetof(struct S2XSetModeration, s2xso_TxMicros) == H + 4);
A(sizeof(struct S2XSetFlowControl) == H + 8 && offsetof(struct S2XSetFlowControl, s2xsf_Modes) == H);
A(sizeof(struct S2XSetRSS) == H + 8 + 2 * P && offsetof(struct S2XSetRSS, s2xsr_Queues) == H + 4 && offsetof(struct S2XSetRSS, s2xsr_TableSize) == H + 6);
A(offsetof(struct S2XSetRSS, s2xsr_Key) == H + 8 && offsetof(struct S2XSetRSS, s2xsr_Table) == H + 8 + P);
/* Not on an official SANA-II command */
A(S2X_QUERY > S2_SANA2HOOK && S2X_CONFIGURE < 0xC200);

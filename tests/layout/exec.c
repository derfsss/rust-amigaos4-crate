#include <exec/exec.h>
#include <exec/avl.h>
#include <exec/debug.h>
#include <utility/hooks.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
A(offsetof(struct Node, ln_Type) == 8);
A(offsetof(struct Node, ln_Pri) == 9);
A(offsetof(struct Node, ln_Name) == 10);
A(sizeof(struct Node) == 14);
A(sizeof(struct List) == 14);
A(offsetof(struct List, lh_TailPred) == 8);
A(offsetof(struct List, lh_Type) == 12);
A(offsetof(struct Message, mn_ReplyPort) == 14);
A(offsetof(struct Message, mn_Length) == 18);
A(sizeof(struct Message) == 20);
A(offsetof(struct MsgPort, mp_SigBit) == 15);
A(offsetof(struct MsgPort, mp_SigTask) == 16);
A(sizeof(struct MsgPort) == 34);
A(offsetof(struct IORequest, io_Device) == 20);
A(offsetof(struct IORequest, io_Unit) == 24);
A(offsetof(struct IORequest, io_Command) == 28);
A(offsetof(struct IORequest, io_Flags) == 30);
A(offsetof(struct IORequest, io_Error) == 31);
A(sizeof(struct IORequest) == 32);
A(offsetof(struct IOStdReq, io_Actual) == 32);
A(offsetof(struct IOStdReq, io_Length) == 36);
A(offsetof(struct IOStdReq, io_Data) == 40);
A(offsetof(struct IOStdReq, io_Offset) == 44);
A(sizeof(struct IOStdReq) == 48);
A(offsetof(struct Library, lib_Version) == 20);
A(offsetof(struct Library, lib_Revision) == 22);
A(offsetof(struct Library, lib_IdString) == 24);
A(offsetof(struct Hook, h_Entry) == 8);
A(offsetof(struct Hook, h_SubEntry) == 12);
A(offsetof(struct Hook, h_Data) == 16);
A(sizeof(struct Hook) == 20);
A(sizeof(struct AVLNode) == 16);
A(offsetof(struct DebugSymbol, Name) == 8);
A(offsetof(struct DebugSymbol, Offset) == 12);
A(offsetof(struct DebugSymbol, SegmentNumber) == 16);
A(offsetof(struct DebugSymbol, SegmentOffset) == 20);
A(offsetof(struct DebugSymbol, SourceFileName) == 24);
A(offsetof(struct DebugSymbol, SourceLineNumber) == 28);
A(offsetof(struct DebugSymbol, SourceFunctionName) == 32);
A(offsetof(struct StackFrameMsg, State) == 4);
A(offsetof(struct StackFrameMsg, MemoryAddress) == 8);
A(offsetof(struct StackFrameMsg, StackPointer) == 12);
A(sizeof(struct SNMMessage) == 20);
int z;
/* Procure bids, low-memory handlers (src/sync.rs, src/interrupt.rs). */
A(offsetof(struct SignalSemaphore, ss_Owner) == 40);
A(offsetof(struct SemaphoreMessage, ssm_Semaphore) == 20);
A(sizeof(struct SemaphoreMessage) == 24);
A(offsetof(struct MemHandlerData, memh_RequestFlags) == 4);
A(offsetof(struct MemHandlerData, memh_Flags) == 8);
A(sizeof(struct MemHandlerData) == 12);
/* track.rs: the panic backtrace bounds its walk by the task stack. */
A(offsetof(struct Task, tc_SPLower) == 58 && offsetof(struct Task, tc_SPUpper) == 62);

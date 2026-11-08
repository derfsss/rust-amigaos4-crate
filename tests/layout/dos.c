#include <dos/dos.h>
#include <dos/rdargs.h>
#include <dos/datetime.h>
#include <exec/tasks.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
/* sys::DateStamp */
A(sizeof(struct DateStamp) == 12);
A(offsetof(struct DateStamp, ds_Minute) == 4);
A(offsetof(struct DateStamp, ds_Tick) == 8);
/* sys::ExamineData (pack(2)) */
A(sizeof(struct ExamineData) == 112);
A(offsetof(struct ExamineData, EXDinfo) == 8);
A(offsetof(struct ExamineData, FSPrivate) == 16);
A(offsetof(struct ExamineData, StructSize) == 24);
A(offsetof(struct ExamineData, Type) == 28);
A(offsetof(struct ExamineData, FileSize) == 32);
A(offsetof(struct ExamineData, Date) == 40);
A(offsetof(struct ExamineData, RefCount) == 52);
A(offsetof(struct ExamineData, ObjectID) == 56);
A(offsetof(struct ExamineData, Name) == 64);
A(offsetof(struct ExamineData, Comment) == 72);
A(offsetof(struct ExamineData, Link) == 80);
A(offsetof(struct ExamineData, Protection) == 88);
A(offsetof(struct ExamineData, OwnerUID) == 92);
A(offsetof(struct ExamineData, OwnerGID) == 96);
/* sys::CSource, sys::RDArgs (pack(2)) */
A(sizeof(struct CSource) == 12);
A(offsetof(struct CSource, CS_CurChr) == 8);
A(sizeof(struct RDArgs) == 32);
A(offsetof(struct RDArgs, RDA_Buffer) == 16);
A(offsetof(struct RDArgs, RDA_Flags) == 28);
/* sys::InfoData, sys::DateTime */
A(sizeof(struct InfoData) == 36);
A(offsetof(struct InfoData, id_DiskState) == 8);
A(offsetof(struct InfoData, id_BytesPerBlock) == 20);
A(offsetof(struct InfoData, id_DOSType) == 24);
A(sizeof(struct DateTime) == 26);
A(offsetof(struct DateTime, dat_Format) == 12);
A(offsetof(struct DateTime, dat_StrDay) == 14);
A(offsetof(struct DateTime, dat_StrTime) == 22);
/* dos::process reads tc_UserData (NP_UserData) at 88 */
A(offsetof(struct Task, tc_UserData) == 88);
/* dos::is_process reads tc_Node.ln_Type at 8 */
A(offsetof(struct Task, tc_Node) == 0);
A(offsetof(struct Node, ln_Type) == 8);

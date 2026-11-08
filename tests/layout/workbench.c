#include <workbench/workbench.h>
#include <workbench/startup.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
#define SZ(s, f) sizeof(((struct s *)0)->f)
/* workbench.rs: struct AppMessage, struct WBArg */
A(sizeof(struct AppMessage) == 86);
A(offsetof(struct AppMessage, am_Type) == 20 && SZ(AppMessage, am_Type) == 2);
A(offsetof(struct AppMessage, am_UserData) == 22 && offsetof(struct AppMessage, am_ID) == 26 && SZ(AppMessage, am_ID) == 4);
A(offsetof(struct AppMessage, am_NumArgs) == 30 && SZ(AppMessage, am_NumArgs) == 4 && offsetof(struct AppMessage, am_ArgList) == 34);
A(offsetof(struct AppMessage, am_Class) == 40 && SZ(AppMessage, am_Class) == 2);
A(offsetof(struct AppMessage, am_MouseX) == 42 && offsetof(struct AppMessage, am_MouseY) == 44 && SZ(AppMessage, am_MouseX) == 2);
A(sizeof(struct WBArg) == 8 && offsetof(struct WBArg, wa_Lock) == 0 && offsetof(struct WBArg, wa_Name) == 4);
A(AMTYPE_APPWINDOW == 7 && AMTYPE_APPICON == 8 && AMTYPE_APPMENUITEM == 9 && AMTYPE_APPWINDOWZONE == 10);
A(WBTOOL == 3 && AMCLASSICON_Open == 0);
/* workbench.rs WbStartup: struct WBStartup, struct Process, struct DiskObject. */
#include <dos/dosextens.h>
#include <dos/dos.h>
#include <exec/nodes.h>
A(offsetof(struct WBStartup, sm_Segment) == 24 && offsetof(struct WBStartup, sm_NumArgs) == 28 && SZ(WBStartup, sm_NumArgs) == 4);
A(offsetof(struct WBStartup, sm_ToolWindow) == 32 && offsetof(struct WBStartup, sm_ArgList) == 36);
A(offsetof(struct Node, ln_Type) == 8 && NT_PROCESS == 13);
A(offsetof(struct Process, pr_MsgPort) == 92 && offsetof(struct Process, pr_CLI) == 172);
A(offsetof(struct Process, pr_Flags) == 192 && SZ(Process, pr_Flags) == 4);
A(PRF_FREESEGLIST == 1 && GPSLF_SEG == (1 << 2));
A(offsetof(struct DiskObject, do_ToolTypes) == 54);

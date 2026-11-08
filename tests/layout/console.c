#include <devices/console.h>
#include <devices/conunit.h>
#include <devices/inputevent.h>
#include <exec/io.h>
#include <intuition/intuition.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
/* console.rs */
A(CON_Dummy == TAG_USER + 0x10000 && CD_GETATTRS == 17 && CMD_READ == 2 && CMD_WRITE == 3 && CMD_CLEAR == 5);
A(CDT_GETROWS == CON_Dummy + 102 && CDT_GETCOLUMNS == CON_Dummy + 103 && CDT_GETYCP == CON_Dummy + 104 && CDT_GETXCP == CON_Dummy + 105);
A(CONU_LIBRARY == -1 && CONU_STANDARD == 0 && CONU_CHARMAP == 1 && CONU_SNIPMAP == 3 && CONFLAG_CREATE_WINDOW == 0x04);
A(sizeof(struct InputEvent) == 22 && offsetof(struct InputEvent, ie_Class) == 4 && offsetof(struct InputEvent, ie_Code) == 6 && offsetof(struct InputEvent, ie_Qualifier) == 8);
A(IECLASS_RAWKEY == 1 && sizeof(struct IOStdReq) == 48);
A(WA_Left == TAG_USER + 100 && WA_Top == TAG_USER + 101 && WA_Width == TAG_USER + 102 && WA_Height == TAG_USER + 103 && WA_Flags == TAG_USER + 107 && WA_Title == TAG_USER + 110);
A(WFLG_SIZEGADGET == 1 && WFLG_DRAGBAR == 2 && WFLG_DEPTHGADGET == 4 && WFLG_ACTIVATE == 0x1000);

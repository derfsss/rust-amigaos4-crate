#include <intuition/intuition.h>
#include <intuition/screens.h>
#include <exec/ports.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
/* intuition.rs: struct Window fields it reads */
A(offsetof(struct Window, LeftEdge) == 4 && offsetof(struct Window, TopEdge) == 6);
A(offsetof(struct Window, Width) == 8 && offsetof(struct Window, Height) == 10);
A(offsetof(struct Window, RPort) == 50);
A(offsetof(struct Window, BorderLeft) == 54 && offsetof(struct Window, BorderTop) == 55);
A(offsetof(struct Window, BorderRight) == 56 && offsetof(struct Window, BorderBottom) == 57);
A(offsetof(struct Window, UserPort) == 86);
/* struct Screen */
A(offsetof(struct Screen, Width) == 12 && offsetof(struct Screen, Height) == 14 && offsetof(struct Screen, Title) == 22);
/* struct IntuiMessage */
A(offsetof(struct IntuiMessage, Class) == 20 && offsetof(struct IntuiMessage, Code) == 24);
A(offsetof(struct IntuiMessage, Qualifier) == 26 && offsetof(struct IntuiMessage, IAddress) == 28);
A(offsetof(struct IntuiMessage, MouseX) == 32 && offsetof(struct IntuiMessage, MouseY) == 34);
A(offsetof(struct IntuiMessage, Seconds) == 36 && offsetof(struct IntuiMessage, Micros) == 40);
A(offsetof(struct MsgPort, mp_SigBit) == 15);
/* struct EasyStruct it builds */
A(sizeof(struct EasyStruct) == 28);
A(offsetof(struct EasyStruct, es_Title) == 8 && offsetof(struct EasyStruct, es_TextFormat) == 12 && offsetof(struct EasyStruct, es_GadgetFormat) == 16);
/* WA_* tags */
A(WA_Left == TAG_USER + 100 && WA_Top == TAG_USER + 101 && WA_IDCMP == TAG_USER + 106 && WA_Title == TAG_USER + 110);
A(WA_MinWidth == TAG_USER + 114 && WA_MinHeight == TAG_USER + 115 && WA_MaxWidth == TAG_USER + 116 && WA_MaxHeight == TAG_USER + 117);
A(WA_InnerWidth == TAG_USER + 118 && WA_InnerHeight == TAG_USER + 119 && WA_PubScreen == TAG_USER + 121);
A(WA_SizeGadget == TAG_USER + 129 && WA_DragBar == TAG_USER + 130 && WA_DepthGadget == TAG_USER + 131 && WA_CloseGadget == TAG_USER + 132);
A(WA_NoCareRefresh == TAG_USER + 135 && WA_Activate == TAG_USER + 137 && WA_RMBTrap == TAG_USER + 138 && WA_SmartRefresh == TAG_USER + 141);
/* IDCMP classes, codes, qualifiers */
A(IDCMP_NEWSIZE == 0x2 && IDCMP_REFRESHWINDOW == 0x4 && IDCMP_MOUSEBUTTONS == 0x8 && IDCMP_MOUSEMOVE == 0x10);
A(IDCMP_CLOSEWINDOW == 0x200 && IDCMP_RAWKEY == 0x400 && IDCMP_ACTIVEWINDOW == 0x40000 && IDCMP_INACTIVEWINDOW == 0x80000);
A(IDCMP_VANILLAKEY == 0x200000 && IDCMP_INTUITICKS == 0x400000 && IDCMP_CHANGEWINDOW == 0x2000000);
A(SELECTDOWN == 104 && SELECTUP == 232 && MENUDOWN == 105 && MENUUP == 233);
A(IEQUALIFIER_LSHIFT == 1 && IEQUALIFIER_RSHIFT == 2 && IEQUALIFIER_CONTROL == 8 && IEQUALIFIER_LALT == 16);
/* intuition.rs menus (GadTools) and refresh */
#include <libraries/gadtools.h>
A(sizeof(struct NewMenu) == 20);
A(offsetof(struct NewMenu, nm_Label) == 2 && offsetof(struct NewMenu, nm_CommKey) == 6);
A(offsetof(struct NewMenu, nm_Flags) == 10 && offsetof(struct NewMenu, nm_UserData) == 16);
A(NM_TITLE == 1 && NM_ITEM == 2 && NM_SUB == 3 && NM_END == 0);
A(offsetof(struct MenuItem, Flags) == 12 && offsetof(struct MenuItem, NextSelect) == 32 && sizeof(struct MenuItem) == 34);
A(MENUNULL == 0xFFFF && IDCMP_MENUPICK == 0x100 && CHECKIT == 1 && MENUTOGGLE == 8 && CHECKED == 0x100);
A(WA_SimpleRefresh == TAG_USER + 140 && GTMN_NewLookMenus == TAG_USER + 0x80043);
/* set_menu_checked walks the strip */
A(offsetof(struct Menu, NextMenu) == 0 && offsetof(struct Menu, FirstItem) == 18);
A(offsetof(struct MenuItem, NextItem) == 0 && offsetof(struct MenuItem, SubItem) == 28);
A(WA_BusyPointer == TAG_USER + 152 && WA_PointerDelay == TAG_USER + 153);
A(sizeof(struct Window) == 136);

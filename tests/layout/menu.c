#include <intuition/menuclass.h>
#include <intuition/intuition.h>
#include <intuition/gadgetclass.h>
#include <classes/window.h>
#include <classes/popupmenu.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
#define U(n, v) A((n) == TAG_USER + (v))
/* menu.rs: menuclass tags, types, methods, state bits. */
U(MA_Type, 0x440001); U(MA_Label, 0x440002); U(MA_Separator, 0x440004); U(MA_ID, 0x440005);
U(MA_Key, 0x440006); U(MA_Disabled, 0x440008); U(MA_Toggle, 0x440009); U(MA_MX, 0x44000a);
U(MA_Selected, 0x44000b); U(MA_AddChild, 0x440032);
A(T_ROOT == -1 && T_MENU == 0 && T_ITEM == 1 && NO_MENU_ID == 0);
A(MM_FINDID == 4001 && MM_SETSTATE == 4006 && MM_GETSTATE == 4007 && MM_NEXTSELECT == 4009);
A(MS_CHECKED == 1 && MS_DISABLED == 2);
A(sizeof(struct mpNextSelect) == 12 && offsetof(struct mpNextSelect, mpns_CurrentID) == 8);
A(sizeof(struct mpGetState) == 12 && offsetof(struct mpGetState, mpgs_ID) == 8);
A(sizeof(struct mpSetState) == 20 && offsetof(struct mpSetState, mpss_ID) == 8);
A(offsetof(struct mpSetState, mpss_ApplyMask) == 12 && offsetof(struct mpSetState, mpss_StateMask) == 16);
A(sizeof(struct mpFindID) == 12 && offsetof(struct mpFindID, mpfi_ID) == 8);
/* intuition.rs: a menuclass strip and context menu on a window. */
U(WA_ContextMenuHook, 99 + 0x52); U(WA_NewLookMenus, 99 + 0x30); U(WA_MenuStrip, 99 + 0x51);
A(offsetof(struct ContextMenuMsg, State) == 0 && offsetof(struct ContextMenuMsg, Menu) == 4 && offsetof(struct ContextMenuMsg, Context) == 8);
A(offsetof(struct Window, MouseY) == 12 && offsetof(struct Window, MouseX) == 14);
A(offsetof(struct IntuiMessage, IAddress) == 28);
/* reaction.rs: window menus and gadget context menus. */
U(WINDOW_MenuStrip, 0x05025004); U(WINDOW_MenuAddress, 0x05025041); U(WINDOW_MenuType, 0x05025042);
U(GA_ContextMenu, 0x30039);
A(WMHI_MENUPICK == (6L << 16) && IDCMP_MENUPICK == 0x100);
/* popup.rs: popupmenu.class. */
U(PMA_AddItem, 0x05025901); U(PMIA_Title, 0x05025932); U(PMIA_ID, 0x05025934); U(PMIA_SubMenu, 0x05025935);
U(PMIA_CheckIt, 0x05025938); U(PMIA_Checked, 0x05025939); U(PMIA_Disabled, 0x0502593d);
U(PMIA_CommKey, 0x05025943); U(PMIA_Toggle, 0x0502594b); U(PMIA_WideTitleBar, 0x05025950);
A(PM_OPEN == 0x571000);
A(sizeof(struct pmOpen) == 8 && offsetof(struct pmOpen, Window) == 4);

#include <libraries/application.h>
#include <exec/ports.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
/* application.rs: message, node and port offsets */
A(offsetof(struct ApplicationMsg, senderAppID) == 20);
A(offsetof(struct ApplicationMsg, type) == 24);
A(offsetof(struct ApplicationOpenPrintDocMsg, fileName) == 28);
A(offsetof(struct ApplicationCustomMsg, customMsg) == 28);
A(sizeof(struct ApplicationCustomMsg) <= 32);
A(offsetof(struct ApplicationNode, appID) == 8);
A(offsetof(struct ApplicationNode, name) == 12);
A(offsetof(struct MsgPort, mp_SigBit) == 15);
/* application.rs constants */
A(REGAPP_UniqueApplication == TAG_USER + 1 && REGAPP_URLIdentifier == TAG_USER + 23);
A(REGAPP_NoIcon == TAG_USER + 30 && REGAPP_Hidden == TAG_USER + 31 && REGAPP_Description == TAG_USER + 50);
A(UNREGAPP_WaitForUnlock == TAG_USER + 1);
A(FINDAPP_Name == TAG_USER + 1 && FINDAPP_AppIdentifier == TAG_USER + 3);
A(APPATTR_Port == TAG_USER + 1 && APPATTR_AppIdentifier == TAG_USER + 5);
A(APPLIBMT_AppRegister == 1 && APPLIBMT_AppUnregister == 2 && APPLIBMT_AppDidChangeIconType == 4);
A(APPLIBMT_AppDidChangeHidden == 5 && APPLIBMT_LastAppDocChange == 6);
A(APPLIBMT_Quit == 100 && APPLIBMT_ForceQuit == 101 && APPLIBMT_Hide == 102 && APPLIBMT_Unhide == 103);
A(APPLIBMT_ToFront == 104 && APPLIBMT_Unique == 105 && APPLIBMT_OpenPrefs == 200 && APPLIBMT_ReloadPrefs == 201);
A(APPLIBMT_NewBlankDoc == 300 && APPLIBMT_OpenDoc == 301 && APPLIBMT_PrintDoc == 302);
A(APPLIBMT_BlankerAllow == 400 && APPLIBMT_BlankerDisallow == 401 && APPLIBMT_ConfirmClose == 470);
A(APPLIBMT_GameModeEntered == 500 && APPLIBMT_GameModeLeft == 501 && APPLIBMT_CustomMsg == 9999);
/* prefs.rs constants */
A(ALPO_Alloc == TAG_USER + 1 && ALPO_Release == TAG_USER + 2000 && ALPO_Identify == TAG_USER + 20000);
A(ALPOSTR_AllocSetString == TAG_USER + 30000 && ALPOSTR_GetString == TAG_USER + 40002);
A(ALPONUM_AllocSetLong == TAG_USER + 30000 && ALPONUM_AllocSetBool == TAG_USER + 30001 && ALPONUM_GetLong == TAG_USER + 40002);
A(ALPODICT_GetCount == TAG_USER + 40000 && ALPODICT_RemoveObjForKey == TAG_USER + 40012);
A(ALPOARR_GetCount == TAG_USER + 40001 && ALPOARR_AddObj == TAG_USER + 40010 && ALPOARR_GetObjAtIndex == TAG_USER + 40012);
A(READPREFS_FileName == TAG_USER + 2 && WRITEPREFS_FileName == TAG_USER + 2);
A(ALPOT_String == 1 && ALPOT_Number == 2 && ALPOT_Array == 5 && ALPOT_Dictionary == 6);
A(offsetof(struct ALPOObjIndex, index) == 4);

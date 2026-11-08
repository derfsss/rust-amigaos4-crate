#include <libraries/asl.h>
#include <workbench/startup.h>
#include <graphics/text.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
#define SZ(s, f) sizeof(((struct s *)0)->f)
/* asl.rs: the requesters' result fields, the tags it passes. */
A(offsetof(struct FileRequester, fr_File) == 4 && offsetof(struct FileRequester, fr_Drawer) == 8);
A(offsetof(struct FileRequester, fr_NumArgs) == 32 && SZ(FileRequester, fr_NumArgs) == 4);
A(offsetof(struct FileRequester, fr_ArgList) == 36);
A(sizeof(struct WBArg) == 8 && offsetof(struct WBArg, wa_Name) == 4);
A(offsetof(struct FontRequester, fo_Attr) == 8);
A(offsetof(struct TextAttr, ta_Name) == 0 && offsetof(struct TextAttr, ta_YSize) == 4 && SZ(TextAttr, ta_YSize) == 2);
A(offsetof(struct TextAttr, ta_Style) == 6 && offsetof(struct TextAttr, ta_Flags) == 7);
A(offsetof(struct ScreenModeRequester, sm_DisplayID) == 0 && SZ(ScreenModeRequester, sm_DisplayID) == 4);
A(offsetof(struct ScreenModeRequester, sm_DisplayWidth) == 4 && SZ(ScreenModeRequester, sm_DisplayWidth) == 4);
A(offsetof(struct ScreenModeRequester, sm_DisplayHeight) == 8 && SZ(ScreenModeRequester, sm_DisplayHeight) == 4);
A(offsetof(struct ScreenModeRequester, sm_DisplayDepth) == 12 && SZ(ScreenModeRequester, sm_DisplayDepth) == 2);
A(offsetof(struct ScreenModeRequester, sm_OverscanType) == 14 && SZ(ScreenModeRequester, sm_OverscanType) == 2);
A(offsetof(struct ScreenModeRequester, sm_AutoScroll) == 16 && SZ(ScreenModeRequester, sm_AutoScroll) == 2);
A(ASL_FileRequest == 0 && ASL_FontRequest == 1 && ASL_ScreenModeRequest == 2);
#define ASL_TB (TAG_USER + 0x80000)
A(ASLFR_TitleText == 0x80080001);
A(ASLFR_TitleText == ASL_TB + 1 && ASLFR_Window == ASL_TB + 2);
A(ASLFR_InitialLeftEdge == ASL_TB + 3 && ASLFR_InitialTopEdge == ASL_TB + 4);
A(ASLFR_InitialWidth == ASL_TB + 5 && ASLFR_InitialHeight == ASL_TB + 6);
A(ASLFR_PubScreenName == ASL_TB + 41 && ASLFR_SleepWindow == ASL_TB + 43);
A(ASLFO_TitleText == ASL_TB + 1 && ASLFO_Window == ASL_TB + 2 && ASLFO_PubScreenName == ASL_TB + 41 && ASLFO_SleepWindow == ASL_TB + 43);
A(ASLSM_TitleText == ASL_TB + 1 && ASLSM_Window == ASL_TB + 2 && ASLSM_PubScreenName == ASL_TB + 41 && ASLSM_SleepWindow == ASL_TB + 43);
A(ASLFO_InitialLeftEdge == ASL_TB + 3 && ASLSM_InitialLeftEdge == ASL_TB + 3 && ASLFO_InitialHeight == ASL_TB + 6 && ASLSM_InitialHeight == ASL_TB + 6);
A(ASLFR_InitialFile == ASL_TB + 8 && ASLFR_InitialDrawer == ASL_TB + 9 && ASLFR_InitialPattern == ASL_TB + 10);
A(ASLFR_PositiveText == ASL_TB + 18);
A(ASLFR_DoSaveMode == ASL_TB + 44 && ASLFR_DoMultiSelect == ASL_TB + 45 && ASLFR_DoPatterns == ASL_TB + 46);
A(ASLFR_DrawersOnly == ASL_TB + 47 && ASLFR_RejectIcons == ASL_TB + 60);
A(ASLFO_InitialName == ASL_TB + 10 && ASLFO_InitialSize == ASL_TB + 11);
A(ASLFO_MinHeight == ASL_TB + 16 && ASLFO_MaxHeight == ASL_TB + 17 && ASLFO_FixedWidthOnly == ASL_TB + 48);
A(ASLSM_InitialDisplayID == ASL_TB + 100 && ASLSM_DoWidth == ASL_TB + 109 && ASLSM_DoHeight == ASL_TB + 110 && ASLSM_DoDepth == ASL_TB + 111);

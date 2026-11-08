#include <classes/window.h>
#include <gadgets/layout.h>
#include <gadgets/button.h>
#include <gadgets/string.h>
#include <gadgets/checkbox.h>
#include <gadgets/integer.h>
#include <gadgets/slider.h>
#include <gadgets/chooser.h>
#include <gadgets/listbrowser.h>
#include <gadgets/fuelgauge.h>
#include <images/label.h>
#include <images/bevel.h>
#include <intuition/gadgetclass.h>
#include <intuition/icclass.h>
#include <intuition/intuition.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
#define U(n, v) A((n) == TAG_USER + (v))
/* reaction.rs: tags, methods and results. */
U(WINDOW_SigMask, 0x05025002); U(WINDOW_Layout, 0x05025005); U(WINDOW_Position, 0x0502500e);
A(WPOS_CENTERSCREEN == 1);
A(WM_HANDLEINPUT == 0x570001 && WM_OPEN == 0x570002);
A(sizeof(struct wmHandle) == 8 && offsetof(struct wmHandle, wmh_Code) == 4);
U(LAYOUT_Orientation, 0x05007001); U(LAYOUT_EvenSize, 0x05007007); U(LAYOUT_BevelStyle, 0x0500700f);
U(LAYOUT_Label, 0x05007010); U(LAYOUT_AddChild, 0x05007014); U(LAYOUT_SpaceInner, 0x05007025); U(LAYOUT_SpaceOuter, 0x05007026);
A(LAYOUT_ORIENT_HORIZ == 0 && LAYOUT_ORIENT_VERT == 1 && BVS_GROUP == 2);
U(CHILD_MinWidth, 0x05007101); U(CHILD_MinHeight, 0x05007102); U(CHILD_WeightedWidth, 0x05007105);
U(CHILD_WeightedHeight, 0x05007106); U(CHILD_Label, 0x0500710c);
U(GA_Text, 0x30009); U(GA_Disabled, 0x3000e); U(GA_ID, 0x30010); U(GA_Selected, 0x30013);
U(GA_RelVerify, 0x30016); U(GA_TabCycle, 0x30024);
U(BUTTON_PushButton, 0x04000001);
U(STRINGA_MaxChars, 0x32001); U(STRINGA_TextVal, 0x32012);
A(CHECKBOX_Checked == GA_Selected);
U(INTEGER_Number, 0x05002001); U(INTEGER_Minimum, 0x05002003); U(INTEGER_Maximum, 0x05002004); U(INTEGER_Arrows, 0x05002005);
U(SLIDER_Min, 0x05028001); U(SLIDER_Max, 0x05028002); U(SLIDER_Level, 0x05028003); U(SLIDER_Orientation, 0x05028004);
A(SORIENT_HORIZ == 2 && SORIENT_VERT == 4);
U(CHOOSER_PopUp, 0x05001001); U(CHOOSER_Active, 0x05001005); U(CHOOSER_LabelArray, 0x0500100c);
U(LISTBROWSER_Labels, 0x05003003); U(LISTBROWSER_Selected, 0x05003004); U(LISTBROWSER_MakeVisible, 0x05003009);
U(LISTBROWSER_ShowSelected, 0x05003012);
U(LBNA_Column, 0x05003504); U(LBNCA_Text, 0x05003505); U(LBNCA_CopyText, 0x0500350f);
U(FUELGAUGE_Min, 0x05012001); U(FUELGAUGE_Max, 0x05012002); U(FUELGAUGE_Level, 0x05012003); U(FUELGAUGE_Percent, 0x05012005);
U(LABEL_Text, 0x05006001);
U(WA_Left, 0x64); U(WA_Top, 0x65); U(WA_IDCMP, 0x6a); U(WA_Title, 0x6e); U(WA_InnerWidth, 0x76);
U(WA_InnerHeight, 0x77); U(WA_PubScreen, 0x79); U(WA_SizeGadget, 0x81); U(WA_DragBar, 0x82);
U(WA_DepthGadget, 0x83); U(WA_CloseGadget, 0x84); U(WA_Activate, 0x89);
A(IDCMP_NEWSIZE == 0x2 && IDCMP_GADGETUP == 0x40 && IDCMP_CLOSEWINDOW == 0x200 && IDCMP_RAWKEY == 0x400 && IDCMP_VANILLAKEY == 0x200000);
A(WMHI_IGNORE == ~0UL && WMHI_CLASSMASK == 0xffff0000UL);
A(WMHI_CLOSEWINDOW == (1L << 16) && WMHI_GADGETUP == (2L << 16) && WMHI_INACTIVE == (3L << 16) && WMHI_ACTIVE == (4L << 16));
A(WMHI_NEWSIZE == (5L << 16) && WMHI_RAWKEY == (11L << 16) && WMHI_VANILLAKEY == (12L << 16));
/* reaction.rs: tequila-style windows (iconify, drops, space, screen title). */
#include <gadgets/space.h>
U(WINDOW_AppPort, 0x0502500d); U(WINDOW_IconTitle, 0x05025012); U(WINDOW_AppMsgHook, 0x05025013);
U(WINDOW_AppWindow, 0x05025015); U(WINDOW_IconifyGadget, 0x05025017);
A(WPOS_CENTERMOUSE == 2 && WM_ICONIFY == 0x570005);
A(WMHI_ICONIFY == (9L << 16) && WMHI_UNICONIFY == (10L << 16));
U(SPACE_Transparent, 0x05009005); U(SPACE_RenderBox, 0x0500900a);
U(WA_ScreenTitle, 0x6f); U(WA_SmartRefresh, 141); U(WA_SimpleRefresh, 140); U(WA_BusyPointer, 152); U(WA_PointerDelay, 153);
A(sizeof(struct IBox) == 8 && offsetof(struct IBox, Width) == 4);
A(offsetof(struct Window, RPort) == 50 && offsetof(struct Window, LeftEdge) == 4);
U(WINDOW_Qualifier, 0x05025035);
#include <gadgets/speedbar.h>
#include <images/bitmap.h>
U(SPEEDBAR_Buttons, 0x05013001); U(SBNA_Image, 0x10009); U(SBNA_Help, 0x1000b); U(SBNA_Text, 0x10010);
U(SBNA_Separator, 0x10012); U(SBNA_Toggle, 0x1000c); U(SBNA_Selected, 0x1000d); U(SBNA_HintInfo, 0x10013); U(SBNA_ButtonID, 0x10014);
U(BITMAP_SourceFile, 0x05019001); U(BITMAP_Screen, 0x05019002); U(BITMAP_Masking, 0x05019004);
A(offsetof(struct Screen, RastPort) == 84);
U(WINDOW_VertProp, 0x0502501b); U(WINDOW_VertObject, 0x0502501c); U(WINDOW_HorizProp, 0x0502501d); U(WINDOW_HorizObject, 0x0502501e);
U(PGA_Total, 0x31007); U(PGA_Visible, 0x31008); U(PGA_Top, 0x31009); U(ICA_TARGET, 0x40001);
A(ICTARGET_IDCMP == ~0UL && IDCMP_IDCMPUPDATE == 0x00800000);
A(offsetof(struct Window, MinWidth) == 16 && offsetof(struct Window, MinHeight) == 18);
A(GM_DOMAIN == 7 && GDOMAIN_NOMINAL == 1);
A(sizeof(struct gpDomain) == 28 && offsetof(struct gpDomain, gpd_RPort) == 8 && offsetof(struct gpDomain, gpd_Which) == 12
  && offsetof(struct gpDomain, gpd_Domain) == 16 && offsetof(struct gpDomain, gpd_Attrs) == 24);
U(GA_HintInfo, 0x30038); U(WINDOW_GadgetHelp, 0x05025016);

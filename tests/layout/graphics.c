#include <graphics/rastport.h>
#include <graphics/text.h>
#include <graphics/regions.h>
#include <graphics/gfx.h>
#include <graphics/gfxbase.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
/* graphics.rs: RastPort it allocates and fills */
A(sizeof(struct RastPort) == 100);
A(offsetof(struct RastPort, Layer) == 0);
A(offsetof(struct RastPort, BitMap) == 4);
A(offsetof(struct RastPort, Font) == 52);
/* TextAttr it builds, TextExtent it reads, TextFont metrics */
A(sizeof(struct TextAttr) == 8);
A(offsetof(struct TextAttr, ta_YSize) == 4);
A(offsetof(struct TextAttr, ta_Style) == 6);
A(sizeof(struct TextExtent) == 12);
A(offsetof(struct TextExtent, te_Height) == 2);
A(offsetof(struct TextExtent, te_Extent) == 4);
A(offsetof(struct TextFont, tf_YSize) == 20);
A(offsetof(struct TextFont, tf_XSize) == 24);
A(offsetof(struct TextFont, tf_Baseline) == 26);
/* Region and its rectangles */
A(sizeof(struct Rectangle) == 8);
A(offsetof(struct Rectangle, MaxY) == 6);
A(offsetof(struct Region, bounds) == 0);
A(offsetof(struct Region, RegionRectangle) == 8);
A(offsetof(struct RegionRectangle, Next) == 0);
A(offsetof(struct RegionRectangle, bounds) == 8);
/* constants */
A(BMA_HEIGHT == 0 && BMA_DEPTH == 4 && BMA_WIDTH == 8 && BMA_ISRTG == 16 && BMA_PIXELFORMAT == 19);
A(BMATags_Friend == TAG_USER && BMATags_PixelFormat == TAG_USER + 2 && BMATags_Clear == TAG_USER + 3 && BMATags_UserPrivate == TAG_USER + 17);
A(LBM_BaseAddress == TAG_USER + 1 && LBM_BytesPerRow == TAG_USER + 2 && LBM_PixelFormat == TAG_USER + 3);
A(PIXF_CLUT == 1 && PIXF_R8G8B8 == 2 && PIXF_A8R8G8B8 == 6 && PIXF_R8G8B8A8 == 8 && PIXF_B8G8R8A8 == 9);
A(JAM1 == 0 && JAM2 == 1 && COMPLEMENT == 2 && INVERSVID == 4);

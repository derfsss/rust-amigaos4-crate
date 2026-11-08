#include <graphics/layers.h>
#include <graphics/clip.h>
#include <graphics/rastport.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
/* layers.rs */
A(offsetof(struct RastPort, Layer) == 0);
A(offsetof(struct Layer, rp) == 12 && offsetof(struct Layer, bounds) == 16);
A(offsetof(struct Rectangle, MinY) == 2 && offsetof(struct Rectangle, MaxX) == 4 && offsetof(struct Rectangle, MaxY) == 6);
A(LAYERS_DUMMY == TAG_USER && LAYA_MinX == TAG_USER + 30 && LAYA_MinY == TAG_USER + 31 && LAYA_MaxX == TAG_USER + 32 && LAYA_MaxY == TAG_USER + 33);
A(LAYA_BitMap == TAG_USER + 37 && LAYA_Flags == TAG_USER + 43 && LAYA_Behind == TAG_USER + 45);
A(LAYERSMART == 2);

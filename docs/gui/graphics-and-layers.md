# Drawing: `graphics` and `layers`

Two modules for drawing pixels:

- `graphics` (feature `graphics`, graphics.library 54): off-screen
  bitmaps, a drawing context on them, fonts and clip regions. The same
  drawing type is used for windows.
- `layers` (feature `layers`): overlapping rectangular areas on a bitmap
  of your own, each drawn in its own coordinates, plus a clip region for
  a window.

## Cargo features

```toml
[dependencies]
amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", features = ["graphics"] }
# with layers (which pulls in graphics):
# amigaos4 = { git = "...", features = ["layers"] }
# drawing into windows needs intuition as well:
# amigaos4 = { git = "...", features = ["intuition", "graphics"] }
```

Both modules work in `std` programs and in programs without a C runtime;
they need only `alloc`. The crate opens graphics.library and
layers.library itself.

## A few AmigaOS words

- **BitMap**: a block of pixel memory. A *palette* (CLUT) bitmap stores a
  pen number per pixel; a *true-colour* (RTG, "graphics card style")
  bitmap stores colours directly.
- **Pen**: an index into the screen's palette. Pens mean something only
  on palette bitmaps.
- **RastPort**: the drawing context: the bitmap it draws on, the current
  pens, drawing mode and font. Graphics calls take a RastPort, not a
  bitmap.
- **Layer**: a rectangle on a bitmap that knows what covers it. Windows
  are layers. Drawing through a layer's RastPort is clipped to its
  visible part.
- **Region**: a set of rectangles, used to limit (clip) drawing.

## Why every call is bounds-checked

A RastPort that is not attached to a layer does **not** clip: a line
past the edge of the bitmap writes into whatever memory follows it. The
crate's `RastPort` therefore checks the coordinates of every drawing call
first. Anything that would reach outside the drawable area (a rectangle,
both ends of a line, an ellipse's whole box, a pixel, a pixel array, a
line of text) is refused with `Err(AmigaError::InvalidArgument(..))` and
nothing is drawn. Reversed rectangles (`x1 < x0` or `y1 < y0`) are
refused too.

For a window or a layer the drawable area is its size; the system also
clips to the part that is visible.

## Rectangles: `Rect`

`Rect { x0, y0, x1, y1 }` is *inclusive* on both ends, as the
graphics.library calls take it. `Rect::new(0, 0, 9, 9)` is 10x10 pixels.

| Method | Returns |
|---|---|
| `width()`, `height()` | `x1 - x0 + 1`, `y1 - y0 + 1` |
| `area()` | the pixel count as `i64` (0 when reversed) |

## Bitmaps: `BitMap`

```rust
use amigaos4::graphics::{BitMap, BitMapOptions, PixFmt};

// A 64x48 true-colour bitmap, cleared, whose memory may be locked.
let opts = BitMapOptions { pixel_format: Some(PixFmt::A8R8G8B8), clear: true, user_private: true };
let mut bm = BitMap::new(64, 48, 32, &opts)?;

// An 8-bit palette bitmap.
let mut pal = BitMap::new(80, 24, 8, &BitMapOptions { clear: true, ..Default::default() })?;
```

`BitMap::new(width, height, depth, &options)` accepts sizes from 1 to
16384 and depths from 1 to 32. `BitMapOptions`:

| Field | Meaning |
|---|---|
| `pixel_format` | `Some(PixFmt::..)` for a true-colour bitmap; `None` for a planar or palette bitmap of the given depth |
| `clear` | start with all pixels zero |
| `user_private` | memory the program may lock and address directly; it never moves |

`PixFmt` names the pixel formats this module knows: `Clut` (one byte, a
pen), `R8G8B8` (3 bytes), `A8R8G8B8`, `R8G8B8A8`, `B8G8R8A8` (4 bytes
each). `bytes_per_pixel()` gives the size.

Other `BitMap` methods:

| Method | Does |
|---|---|
| `new_like(&friend, w, h)` | a cleared bitmap in the same format as `friend` |
| `width()`, `height()` | the size asked for |
| `allocated_width()`, `allocated_height()` | the size actually made (may be rounded up) |
| `depth()`, `is_rtg()`, `pixel_format()` | what was made (`pixel_format` is `None` for formats the module does not know) |
| `blit_from(&src, from, x, y)` | copies `src`'s rectangle `from` to `(x, y)` here |
| `blit_within(from, x, y)` | copies within this bitmap (the areas may overlap) |
| `lock()` | direct access to the pixel memory (see below) |
| `rastport()` | a `RastPort` that draws into this bitmap |
| `as_ptr()`, `into_raw()` | the raw pointer; `into_raw` gives ownership away (not freed) |

Blits are bounds-checked like drawing. The bitmap is freed on drop, after
any blit still running has finished.

### Direct pixel access: `lock()`

`bm.lock()` returns `Some(BitMapLock)` for a bitmap whose memory can be
locked (true-colour ones; `None` for planar ones). Keep a lock short: the
system waits for it before it can switch screens.

```rust
if let Some(mut lock) = bm.lock() {
    let bpr = lock.bytes_per_row() as usize;   // row pitch, at least width * 4
    let px = lock.bytes_mut();                  // bytes_per_row * rows
    // A8R8G8B8: pixel (x, y) is px[bpr * y + x * 4 ..][..4] = A, R, G, B
    px[bpr * 10 + 10 * 4 + 1] = 0xFF;            // red channel of (10, 10)
} // unlocked here
```

`pixel_format()` on the lock says how the bytes are laid out.

## Drawing: `RastPort`

Get one from `BitMap::rastport()`, from a window
(`intuition::Window::rastport()` or `reaction::Window::rastport()`), or
from a layer (`Layer::rastport()`). It borrows its owner, so the owner
cannot go away while you draw.

### Pens or colours

Use **pens** on palette bitmaps and **ARGB colours** (`0xAARRGGBB`) on
true-colour ones. Window drawing on a modern screen usually uses the
colour calls. Every call that takes coordinates returns a `Result`
(the types below are what is inside it).

| Pen calls | Colour calls (ARGB) |
|---|---|
| `set_a_pen(pen)`, `set_b_pen(pen)` | (the colour is an argument) |
| `clear(pen)`: whole area | `clear_color(argb)` |
| `fill(rect)`: in the A pen | `fill_color(rect, argb)` |
| `write_pixel(x, y)`: in the A pen | `write_color(x, y, argb)` |
| `read_pixel(x, y) -> Option<u32>`: the pen | `read_color(x, y) -> u32` |
| `line(x0, y0, x1, y1)`: in the A pen | |
| `ellipse(cx, cy, rx, ry)`: in the A pen | |

Also: `set_draw_mode(mode)` (JAM1 0, JAM2 1, COMPLEMENT 2, INVERSVID 4)
and `size()`, the drawable width and height.

`clear` and `clear_color` cover the whole bitmap; on a window's RastPort
that includes the borders, so prefer `fill_color` over `inner()`.

```rust
use amigaos4::graphics::Rect;

let mut rp = pal.rastport()?;
rp.clear(5);
rp.set_a_pen(3);
rp.fill(Rect::new(2, 2, 9, 5))?;
rp.set_a_pen(1);
rp.line(20, 1, 40, 1)?;
assert_eq!(rp.read_pixel(30, 1)?, Some(1));
```

The ellipse's whole box must be inside the area; a negative radius is
refused.

### Pixel arrays

`write_pixels` copies a buffer of pixels into a rectangle;
`read_pixels` copies a rectangle out. Each takes the rectangle, the
buffer's `PixFmt`, the buffer, and its row pitch in bytes. The system
converts between the buffer's format and the bitmap's.

```rust
use amigaos4::graphics::{PixFmt, Rect};

// A 16x16 ARGB picture, 64 bytes per row.
let src = vec![0xFFu8; 16 * 16 * 4];
rp.write_pixels(Rect::new(5, 5, 20, 20), PixFmt::A8R8G8B8, &src, 16 * 4)?;

let mut rgb = vec![0u8; 16 * 16 * 3];
rp.read_pixels(Rect::new(5, 5, 20, 20), PixFmt::R8G8B8, &mut rgb, 16 * 3)?;
```

A buffer too small for the rectangle at that pitch is refused. This is
how a picture viewer draws a decoded image into a window.

### Text and fonts: `Font`

`Font::open(name, size)` opens a font (`"topaz.font"`, 8) and closes it on
drop. The nearest match is returned: check `y_size()`. `x_size()` is the
nominal width (exact for fixed-width fonts) and `baseline()` the distance
from the top to the baseline.

```rust
use amigaos4::graphics::Font;

let topaz = Font::open("topaz.font", 8)?;
let mut rp = pal.rastport()?;
rp.set_font(&topaz);           // the font must outlive rp
rp.set_a_pen(1);
let w = rp.text_width(b"Hello")?;   // 40 pixels in topaz 8
rp.text(2, 4, b"Hello")?;           // top-left corner at (2, 4)
```

- `text(x, y, bytes)`: `(x, y)` is the **top-left** of the text, not the
  baseline. The whole line (its width by the font's height) must fit, or
  the call is refused.
- `text_width(bytes)` and `text_extent(bytes)` measure without drawing.
  `Extent` has `width`, `height` and `bounds` (relative to the pen
  position; `y` negative above the baseline).
- Text is **bytes in the font's character set (Latin-1)**, not UTF-8.
  Pass `b"..."` or `s.as_bytes()` for ASCII text.
- A RastPort without a font set uses the default one. On a window's
  RastPort, `set_font` lasts only while you hold it: the window's own
  font is put back on drop.

## Regions: `Region`

A `Region` is a set of non-overlapping rectangles, freed on drop. Use it
to clip a window's drawing (below), or simply to compute areas.

```rust
use amigaos4::graphics::{Rect, Region};

let mut r = Region::new()?;
r.or_rect(Rect::new(0, 0, 9, 9))?;
r.or_rect(Rect::new(5, 5, 14, 14))?;
assert_eq!(r.area(), 175);
assert_eq!(r.bounds(), Rect::new(0, 0, 14, 14));
r.clear_rect(Rect::new(0, 0, 14, 4))?;
for rect in r.rects() {
    println!("{:?}", rect);
}
```

| Method | Does |
|---|---|
| `or_rect`, `xor_rect`, `clear_rect`, `and_rect` | combine with a rectangle (`clear_rect` subtracts it) |
| `or_region`, `and_region`, `xor_region` | combine with another region |
| `clear()` | empty it |
| `bounds()` | the bounding box (meaningless when empty) |
| `rects()` | the rectangles, in absolute coordinates |
| `area()`, `is_empty()` | covered area |

## Layers on your own bitmap: `layers`

A `LayerSet` manages layers on a bitmap you own; a `Layer` is one
rectangle in it. Layers are *smart refresh*: what another layer covers is
kept and comes back when it is uncovered.

```rust
use amigaos4::graphics::{BitMap, BitMapOptions, PixFmt, Rect};
use amigaos4::layers::{LayerSet, Place};

let opts = BitMapOptions { pixel_format: Some(PixFmt::A8R8G8B8), clear: true, ..Default::default() };
let mut bm = BitMap::new(200, 150, 32, &opts)?;
let set = LayerSet::new(&mut bm)?;

let mut a = set.create(Rect::new(10, 10, 109, 109), Place::Front)?;
let mut b = set.create(Rect::new(50, 50, 149, 129), Place::Front)?;

// Each layer draws in its own coordinates: (0, 0) is its top-left.
a.rastport()?.fill_color(Rect::new(0, 0, 99, 99), 0xFFC0_0000)?;
b.rastport()?.fill_color(Rect::new(0, 0, 99, 79), 0xFF00_00C0)?;

a.to_front()?;          // a's hidden part comes back
b.move_by(40, 10)?;     // moved with its contents
let top = a.is_top_at(60, 60);
let colour = set.read_color(60, 60)?;   // what the bitmap shows there
```

| `LayerSet` | |
|---|---|
| `new(&mut bitmap)` | layers over the whole bitmap (borrowed while the set lives) |
| `create(rect, Place::Front or Place::Back)` | a new layer inside the bitmap |
| `read_color(x, y)` | the colour of the bitmap itself at a point |

| `Layer` | |
|---|---|
| `bounds()` | where it is in the bitmap |
| `move_by(dx, dy)`, `size_by(dw, dh)` | move or resize; must stay inside the bitmap and at least 1x1 |
| `to_front()`, `to_back()` | restack |
| `is_top_at(x, y)` | whether this layer is the top one at a bitmap point |
| `rastport()` | a `RastPort` in the layer's coordinates, limited to its size |

Layers are deleted on drop, and borrow their set, so they go first. The
area no layer covers is painted by the layer system itself; do not expect
it to keep the bitmap's own contents.

## A window's clip region: `Window::clip`

With features `intuition` and `layers`, `Window::clip(&region)` limits a
window's drawing to a region given in window coordinates. Draw through
the guard's `rastport()`; the window's previous clip region comes back
when the guard is dropped.

```rust
use amigaos4::graphics::{Rect, Region};

let (x0, y0, x1, y1) = win.inner();
let mut region = Region::new()?;
region.or_rect(Rect::new(x0, y0, x0 + 49, y0 + 29))?;
{
    let mut guard = win.clip(&region)?;
    let mut rp = guard.rastport()?;
    // fills only the 50x30 corner
    rp.fill_color(Rect::new(x0, y0, x1, y1), 0xFFC0_00C0)?;
}
// here drawing reaches the whole window again
```

Do not hold the guard across a refresh (`Window::refresh`).

## Limitations

- Pens mean something only on palette bitmaps; use the `*_color` calls on
  true-colour bitmaps.
- Text is Latin-1 bytes, not UTF-8.
- A layer's or window's RastPort is limited to its size from its origin;
  the crate does not stop you drawing over a window's borders (keep to
  `inner()`).
- `layers` covers only layers on a bitmap of your own and a window's clip
  region; the system's screen layers are not wrapped.

# imgview

A picture viewer for AmigaOS 4.1, written in Rust with the `amigaos4`
crate: no C runtime, about 165 KB.

```
imgview [FILE] [FIT/S] [SMOOTH/S] [PUBSCREEN/K]
```

It shows any picture a datatype can read (on the stock 4.1 FE image:
BMP, GIF, ILBM, JPEG, PNG, TIFF). It starts from a Shell or from
Workbench: double-click its icon (tool types `FIT`, `SMOOTH`,
`PUBSCREEN=name`), or make it a picture icon's default tool.

| Do this | To |
|---|---|
| `o` (Project > Open..., Amiga-O) | choose a picture in asl's file requester (it starts in the current picture's drawer) |
| drop a file icon on the window | open it |
| space or `n` / backspace or `p` (Project menu, Amiga-N / Amiga-P) | show the next / previous picture in the same directory (files that are not pictures are skipped; the ends beep) |
| `f` (View > Fit to window, Amiga-F) | scale the picture to the window, larger or smaller, keeping its proportions (off: back to the zoom, the window sized for it) |
| `1` (View > Actual size, Amiga-1) | show the picture at 100%, the window sized for it; on its own setting, beside Fit: with both on, 100% unless the picture is larger than the window, which it then fits |
| `l` (View > Lock window size, Amiga-L) | keep the window's size: zooming and other pictures no longer resize it; a picture larger than the window is moved with the border scrollers (or the cursor keys). Off: the window keeps its size until the next zoom, Fit, Actual size or picture, which size it for the picture again |
| `+` / `-` (View > Zoom in / Zoom out) | zoom in steps from 10% to 800%, the window following the size; turns Fit and Actual size off |
| `s` (View > Smooth scaling, Amiga-S) | scale smoothly: an area average when shrinking, bilinear when enlarging (otherwise the nearest pixel) |
| cursor keys (shift: a page) | scroll a picture larger than the window |
| right button over the picture | a context menu: Fit, Smooth, Zoom in/out, Actual size, Open, Next, Previous |
| the iconify gadget | iconify to a Workbench icon (double-click it to come back) |
| `q`, Esc, Amiga-Q, the close gadget, CTRL-C, Quit from AmiDock | quit |

It registers with application.library (Quit, ToFront and OpenDoc
messages) and keeps the window's place and size and the fit and smooth
settings in `ENV:imgview.xml` and `ENVARC:imgview.xml`. Project >
About says how it was started.

## How it is built

`tools/build-package.sh apps/imgview` (on Linux or WSL), like the other
examples: the Rust code is a staticlib linked by `Makefile` with
`src/driver_glue.c`, which opens dos.library and calls `rust_main`. The
crate opens every other library itself.

What it uses from the crate: `workbench::WbStartup` (the Workbench
start, replied without Forbid), `dos::args` (ReadArgs), `dos::fs`
(locks, directories), `datatypes` (`DtObject::read_pixels` into ARGB),
`asl` (the file requester), `menu` (the menu strip and the context menu,
menuclass), `intuition` (window, events, requesters, busy pointer,
AppWindow), `graphics`
(`RastPort::write_pixels`, fills), `application`, `prefs`.

## Not done

Rotation, slideshows, saving or converting pictures, animation.

## Images and icons

- Toolbar images are AISS's (nav_west, nav_east, zoom_out, zoom_in,
  zoom_100, fit, lock, open), looked for in an `Images` drawer beside the
  program, then `TBImages:` (where the AISS installer puts them), then
  Filer's images; a button without one shows text. The eight images
  are in `Images/`, unmodified from AISS 4.22 by Martin "Mason" Merz,
  under CC BY-NC-ND 2.0 DE (see `Images/AISS-LICENCE.txt`): copy the
  drawer beside the program.
- `imgview.info`: the program's icon, PicShow's icon from the Masonicons
  Collection 2009-2010 by Martin "Mason" Merz (freeware,
  http://www.masonicons.de), made a plain tool icon: its tool types and
  position removed, stack 64 KB, images unchanged.
- `drawer.info`: the icon for the drawer imgview is installed in (copy it
  beside the drawer as `imgview.info`): drawer_pictures from the same
  collection, made a drawer icon (type WBDRAWER with DrawerData and the
  DrawerData2 tail, as the collection's own drawer icon has; no fixed
  position), images unchanged.

## Notes

- A truncated PNG is decoded by the PNG datatype as a black picture of
  the header's size, without an error.
- window.class lays the window out after it opens and after each
  resize, so imgview draws only once that layout is done (drawing at
  once used the old picture area and painted over the toolbar).
- The window is never narrower than the toolbar.

## Tested

On QEMU's AmigaOne machine and on an AmigaOne X5000 with AmigaOS 4.1 FE:
started from a Shell and from Workbench; JPEG, BMP, GIF and PNG
pictures; fit, zoom, smooth scaling, scrolling, drops, the menus, the
toolbar, and OpenDoc, ToFront and Quit through
application.library.

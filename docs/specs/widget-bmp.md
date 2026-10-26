# `cGZWinBMP`: the bitmap window (behaviour spec)

Scope: the `cGZWinBMP` owner as tools/coverage.py assigns it (26 functions, 1,407 bytes): the GZ window that shows a
picture (class `0x82FE68C4`, 0xFC bytes, widget-comdirector.md §2), and four small setters of `cWinBMPView` (§5)
that the tool attributes to it.

Related: widget-toolkit.md (`cGZWin`, the draw area `+0x24`, the draw context `+0x6C`), scripted-window-director.md
§4 (the same 3 × 3 edge drawing `RZDrawBltEdge`), view3d-window.md (a user: `SetWinBMPFlag(6, 1)`).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the owner was read in the decompile and the interface vtable
`0x00ADF66C` was read. Written 2026-10-24 (re-gzwinbmp7).

## 1. Layout *[compat]*

| Offset | Field | Constructor (`0x009BC4BA`) |
|---|---|---|
| `+0x00` | `cGZWin`, vtable `0x00ADF6A0` | |
| `+0xD8` | the `cIGZWinBMP` interface (vtable `0x00ADF66C`, IID `0xC12CEA13`) | |
| `+0xDC` | the picture (counted) | null |
| `+0xE0` / `+0xE4` | alpha mode (byte) / alpha (float) | 0 / 0.5 |
| `+0xE8`…`+0xF4` | the source rectangle in the picture | 0 |
| `+0xF8` | bitmap flags (§2) | `0x12` |

`QueryInterface` (`0x009BC06A`, thunk `0x009BC31A`): `0xC12CEA13` → `+0xD8` (null for a null object) with a
reference; else `cGZWin`'s.

## 2. Bitmap flags (`+0xF8`) *[compat]*

| Bit | Effect |
|---|---|
| `0x1` | mouse messages go to the parent (§4) |
| `0x2` | no background fill |
| `0x4` | the window is transparent to the mouse (`IsPointInWindowScreenCoordinates` false) |
| `0x8` | draw the picture as a 3 × 3 edge frame (`RZDrawBltEdge`) |
| `0x10` | use the source rectangle (else the picture's own bounds) |
| `0x20` | use the source rectangle **and** shift the destination by its origin |

The default `0x12` is "source rectangle, no fill".

## 3. The `cIGZWinBMP` interface *[compat]*

| Slot | Address | Behaviour |
|---|---|---|
| 0–3 | | `QueryInterface` thunk, shared `AddRef` / `Release`, `AsIGZWin` |
| 4 `SetBMP(p)` | `0x009BC57E` | when different: assign with reference counting; the source rectangle is reset from the window's area (window slot 48, `SetSourceBufferAreaFromWindowArea`); invalidate |
| 5 `GetBMP(iid, out)` | `0x009BC0BD` | the picture's own `QueryInterface(iid, out)`; false without a picture |
| 6 `SetAlpha(mode, alpha)` | `0x009BC0CF` | store both; invalidate |
| 7 `GetAlpha(&mode, &alpha)` | `0x009BC0EE` | |
| 8 `SetImageArea(r)` | `0x009BC103` | store r; with a picture: L, T raised to 0, R, B lowered to the picture's width and height (picture slots 9, 10) |
| 9 `GetImageArea(&r)` | `0x009BC15F` | |
| 10 `GetWinBMPFlag(mask)` | `0x009BC171` | any bit of `mask` set |
| 11 `SetWinBMPFlag(mask, on)` | `0x009BC181` | set or clear, no redraw |

`SetSourceBufferAreaFromWindowArea(r)` (`0x009BC447`) makes the source rectangle `(0, 0, W, H)` of `r`, clamped to
the picture's size. **`Init`** (`0x009BC52D`, once) calls it with the window's own size, then `cGZWin::Init`.
**`Shutdown`** (`0x009BC097`) releases the picture first. The alpha pair is stored but not used by `GZPaint`.

## 4. Messages and drawing *[compat]*

- **`DoMessage`** (`0x009BC199`): keys (5, 6) go to `cGZWin::DoMessage`. Mouse move, left down and left up (7, 13,
  10) also go to it first. Then, with flag 1 and a parent, mouse messages (7–14) are moved by the window's left and top
  (slots 43, 44, in place) and handed to the parent's `DoMessage`, whose result is returned; otherwise false.
- **Mouse notifications**: `GZOnMouseMove`, `GZOnMouseDownL`, `GZOnMouseUpL` (`0x009BC27C`, `0x009BC2A6`,
  `0x009BC2D0`) send `(0xC89155E3 / 0x68915615 / 0x28916985, x, y, ID)` to the notification target (window slot 144,
  the ID from slot 63).
- **`GZPaint`** (`0x009BC325`):
  1. Without flag 2: the draw area is filled with the fill colour (window slot 126; draw context slots 21, 35).
  2. With a picture: `src` = the source rectangle when flag `0x10` or `0x20` is set, else the picture's bounds
     (slot 12). Without flag 8: `dst` = the draw area's top-left plus `src`'s size, shifted by `src`'s origin when
     flag `0x20` is set; the draw context's slot 38 copies `src` to `dst`. With flag 8: `src`'s right and bottom are
     divided by 3 and `RZDrawBltEdge(context, picture, &src, &drawArea, 1)` draws the frame.
- Destructors: `0x009BC2FA`, deleting `0x009BC511`.

## 5. Four `cWinBMPView` setters in this owner

`cWinBMPView` (a zoomable, scrollable picture view, its own owner) keeps three settings next to its `cGZWinBMP` part:
`0x007BFA10` stores a byte at `+0x100`; `0x007BFA20` a byte at `+0x101`; `0x007C0800` stores `+0x110` and, when it
changed, recomputes the view's metrics (`cWinBMPView::CalculateAllMetrics`) and invalidates; `0x007BFD40` calls the
interface's slot 4 with null (drops the picture).

## 6. Quirks

- `SetWinBMPFlag` never redraws; a flag change shows only at the next invalidation.
- With flag 1 the window passes the mouse to its parent but still runs `cGZWin`'s own handling of moves and left
  presses first.
- The alpha mode and value have no effect on this class's drawing.

## 7. Function inventory

`0x007BFA10`, `0x007BFA20`, `0x007BFD40`, `0x007C0800` (§5); `0x009BC06A`, `0x009BC097`, `0x009BC0BD`,
`0x009BC0CF`, `0x009BC0EE`, `0x009BC103`, `0x009BC15F`, `0x009BC171`, `0x009BC181`, `0x009BC199`, `0x009BC251`,
`0x009BC27C`, `0x009BC2A6`, `0x009BC2D0`, `0x009BC2FA`, `0x009BC31A`, `0x009BC325`, `0x009BC447`, `0x009BC4BA`,
`0x009BC511`, `0x009BC52D`, `0x009BC57E`.

# The Game of Life tool `cSC4ViewInputControlGameOfLife` (behaviour spec)

Scope: the parts of the `GOL` cheat's view-input control (view3d-window.md §3.1) that game-of-life.md leaves out —
construction, Init, Shutdown, destruction, the keys, the mouse and the marked-cell refresh — so that, together with
game-of-life.md (the embedded simulator: storage, Init, RandomizeCells, edits, generation, the tick wrapper and the
live-cell adapter), both owners `cSC4ViewInputControlGameOfLife` and `cSC4GameOfLifeSimulator` are specified. The
base control is city-tools-input.md §2; the marked-cell view is overlay-views.md §2; the tick manager is
animation-tick-manager.md.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the disassembly; the colour constant was read in the PE.
Written 2026-10-25 (re-gol8).

## 1. State *[compat]*

`0x88` bytes, vtable `0x00A90458`, a tick-participant sub-object at `+0x28` (vtable `0x00A90448`). Constructor
`0x004BC310`:

| Offset | Meaning | Constructor |
|---|---|---|
| `+0x0C`, `+0x10` | control id, cursor id | `0x69701127`, 0 |
| `+0x2C` | the terrain, held | null |
| `+0x30`, `+0x34` | the cell width and its inverse | 1.0, 1.0 |
| `+0x38`, `+0x3C` | the last edited cell (x, z) | −1, −1 |
| `+0x40…+0x60` | the simulator (game-of-life.md §2: `+0x44` mode, `+0x48`/`+0x4C` width/height, `+0x50` C, `+0x54` P, `+0x58…+0x60` callbacks) | zero |
| `+0x64` | the marked-cell view, held | null |
| `+0x68…+0x70` | a vector of `(x, z)` cells (8 bytes each) | empty |
| `+0x74` | a string (buffer `+0x78…+0x80`) | empty |

The destructor body (`0x004BC440`) destroys the string and the cell vector, frees C then P (nulling each) and runs
the base destructor; the deleting form (`0x004BC420`) frees when flag bit 0 is set.

## 2. Init (`0x004BC670`) and Shutdown (`0x004BC190`) *[compat]*

**Init**, once: the base `Init`; register `+0x28` with the animation tick manager (slot 5 `AddTickableObject(+0x28,
0xFF)`); when no terrain is held, hold `0x00B43CF4`, `+0x30 :=` its cell width (slot 6) and `+0x34 := 1 / +0x30`. When
no marked-cell view exists: create one (`0x50` bytes), hold it, `Init`, `SetShouldIgnoreDepthBuffer(true)`,
`SetCellSize(w, w)`, `SetShouldUseWaterAltitude(true)`, `SetCellColor({0.0, 0.8, 0.2, 0.7})` (`0x00A90438`), add it
to the renderer (`0x00B43DD0` slot 32 `AddViewObject(view, 5, 1000)`) and call the terrain view's slot 10 with 0
(terrain slot 66). Then, every time: mode `+0x44 := 0`; the simulator's `Init(cellCountX, cellCountZ, 1)` (terrain
slots 7, 8; game-of-life.md §3.1); callbacks: live := the adapter `0x004BBE50`, dead := null, context := this;
**Refresh** (§4); load the string `(group 0x6A231EAA, instance 0x4A56B5FB)` into `+0x74` (nothing here shows it).
Returns true.

**Shutdown**, when initialised: `ClearCursorText(0xA9C87D93)` (3D view slot 41); the marked-cell view: remove it from
the renderer (slot 33), its `Shutdown`, release, clear; the simulator cleanup of game-of-life.md §2; release and clear
the terrain; `RemoveTickableObject(+0x28, 0xFF)` (slot 6); the base `Shutdown`. Returns true.

## 3. Input *[compat]*

- **OnKeyDown(vk, modifiers)** (`0x004BC7D0`): `C` (`0x43`), Delete (`0x2E`) or Backspace (`0x08`): zero P then C
  (`width·height` bytes each, when allocated) and `ClearCells` on the view (no refresh); true. `R` (`0x52`):
  `RandomizeCells` (game-of-life.md §3.3), Refresh, true. Any other key: the negation of `IsKeyToBePassed(vk,
  modifiers)` (the base returns true, so false).
- **OnMouseDownL(x, z, m)** (`0x004BC870`): `PickTerrain(x, z, &p, 0)` (3D view slot 36). On a hit: `SetCapture`; the
  cell `(cx, cz) := (trunc(p.x · +0x34), trunc(p.z · +0x34))` (`_ftol2`), stored in `+0x38/+0x3C`; mode `+0x44 :=
  (C[cz·width + cx] ≠ 0)` (1 erases a live cell, 0 adds; the read is not bounds-checked *[quirk]*); the simulator's
  edit `0x004BBF00(cx, cz)` (game-of-life.md §4); Refresh. Returns true, hit or not.
- **OnMouseMove(x, z, m)** (`0x004BC900`): only while capturing (slot 25): `PickTerrain(x, z, &p, 1)`; on a hit, when
  the cell differs from `+0x38/+0x3C`, store it, edit it with the current mode and Refresh. Returns true.
- **OnMouseUpL** (`0x004BBE70`): `ReleaseCapture` when capturing; true.

## 4. Refresh (`RefreshMarkedCellView`, `0x004BC4D0`) *[compat]*

Empty the cell vector `+0x68`; for z = 0…height−1 (outer) and x = 0…width−1 (inner), walking C, append `(x, z)` for
each non-zero byte. Then empty the view's cells (its vector at `+0x10`) and append the whole vector to it
(`0x004BC3B0`: each 8-byte cell pushed back in order). The generation callbacks add cells one by one instead
(game-of-life.md §5.3).

## 5. Conformance tests

1. Cell width 16: a click at world (40, ·, 70) edits cell (2, 4); clicking a live cell sets mode 1 and a drag over
   further cells erases them.
2. `C` clears both buffers and the marked cells but does not refill them; `R` refills the cells from the new random
   buffer.
3. A 3 × 2 grid with C = `01 00 00 / 00 02 00` refreshes to the cells (0, 0) and (1, 1), in that order.

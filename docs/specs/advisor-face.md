# `cSC4AdvisorUIImageWinProc`: the advisor's 3D face (behaviour spec)

Scope: the `cSC4AdvisorUIImageWinProc` owner as tools/coverage.py assigns it (6 functions, 2,114 bytes): `Init`,
`Shutdown`, the model loader, `DoWinProcMessage`, the animation step and `DoPaint`. The proc draws an advisor's
animated 3D head into its button window, with an orthographic camera, through the 3D renderer. The constructor
(`0x00420B70`) and the small interface methods (§1) belong to other owners; they are described here because the
owned functions depend on them (and they answer AW5).

Related: advisor-window.md (the advisor window creates one face proc per advisor button and calls slots 6, 7, 8),
advisors.md (`get_advisor_mood`, the advisor slot 17 model list, the DollyLlama cheat), draw-context.md
(`SC4DrawContext`), camera.md (`cS3DCamera`), animated-sprite.md (model resources, type `0x5AD0E817`).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the owner was read in the decompile; the loader, `DoPaint`, the
constructor, slots 6–9 and the message target were checked on the instructions, and the vtables `0x00A83AC0` and
`0x00A83AB0` were read. Written 2026-10-24 (re-gzadvface7).

## 1. Object and interface *[compat]*

Constructor `0x00420B70(id)`: everything zero except `+0x44` := the advisor ID and a `cRZRandom` at `+0xD8` (seed
−1).

| Offset | Field |
|---|---|
| `+0x00` / `+0x04` | win proc (vtable `0x00A83AC0`) / message target (vtable `0x00A83AB0`) |
| `+0x08` | reference count |
| `+0x0C` / `+0x0D` | initialised / visible (bytes) |
| `+0x10` | mood index (§2) |
| `+0x14` / `+0x18` | idle model ID / its model (counted) |
| `+0x1C` / `+0x20` | second model ID / its model (counted) |
| `+0x24` | the model being played (counted) |
| `+0x28` | the button window (counted) |
| `+0x2C`…`+0x38` | the window's area L, T, R, B |
| `+0x3C` / `+0x3E` | screen width / height (16-bit) |
| `+0x40` | the camera (`cS3DCamera`, 0x1A8 bytes, counted) |
| `+0x48`…`+0x84` | the camera's projection matrix, transposed |
| `+0xC8` | the playing model's last "still running" value |
| `+0xCC` / `+0xD0` | idle loops played / idle loops before the second model |
| `+0xF0` | the pressed offset (slot 7) |

**Slots** (win proc vtable): 0 `QueryInterface` (`0x00420030`: `0x22E85D8E` → `+0`, `0x452294AA` → `+4`); 3
`DoWinProcMessage`; 5 the deleting destructor (`0x00420C00`); **6 `SetVisible(on)`** (`0x004200A0`); **7
`SetPressedOffset(n)`** (`0x00683EA0`, a shared setter of `+0xF0`); **8 `SetMood(m)`** (`0x00420130`, a jump to slot
9); 9 the mood setter (`0x00420140`); 10 `PlaceModel(x, y, z, s)` (`0x00420C20`); 11 `LoadModels(on)`
(`0x004205D0`); 12 `DoPaint`; 13 `Animate(t)` (`0x00420900`).

- **`SetVisible(on)`**: when it changes, the visible byte is stored; on → the message target is registered with the
  animation tick manager (its slot 5, with `0xC0`) and `LoadModels(1)`; off → unregistered (slot 6) and
  `LoadModels(0)`.
- **`SetMood(m)`**: m is mapped 0 → 3, 1 → 2, 3 → 0, 5 → 1 (2, 4 and anything above 5 unchanged); when the result
  differs from `+0x10`: stored, `+0xCC` := 0, `LoadModels(visible)`.
- **The message target** (`DoMessage` `0x00420100`): message `0x86AD10EE` → `Animate(its slot 19 value)`; true for
  every message.

## 2. Loading the models (`LoadModels`, `0x004205D0`) *[compat]*

- **off**: both model IDs := 0 and both models released (the playing model is kept).
- **on**: the advisor (`sys.GetAdvisor(+0x44)`, city slot 81 then slot 7; none → nothing). Idle ID :=
  advisor slot 17 with index `mood`, second ID := advisor slot 17 with index `mood + 4` (the tables `0x00A83A8C` and
  `0x00A83A9C`). When the idle ID equals the current one nothing more happens. Otherwise both are stored, the old
  models released, and each model is loaded from key `(0x5AD0E817, 0xBADB57F1, ID)`, or first from `ID + 0x100`
  while the DollyLlama byte (`0x00B20D50`, advisors.md) is set, falling back to `ID`. A model is the resource
  (manager slot 3, interface `0x9AD0E7E3`) whose slot 4 `(0, &out)` makes the instance; the instance gets slot 20
  `(1)`.
- Then the idle model becomes the playing model and is restarted (its slot 10 `(0)`).

## 3. Animation (`Animate(t)`, `0x00420900`) *[compat]*

With a playing model: it advances (slot 15 `(t)`). When its slot 11 now reads 0 (finished) and the last value was
not 0:

- if `+0xCC` = 0: `+0xCC` := 1, the idle model plays again from the start, and `+0xD0` := a random integer 1…3
  (`RandomSint32RangeUniform(1, 3)` on `+0xD8`);
- else `+0xCC` += 1; when it reaches `+0xD0`: `+0xCC` := 0 and the second model plays from the start.

Then `+0xC8` := slot 11. So the head loops its idle animation and, after a random 1–3 loops, plays its second
animation once.

## 4. Window messages, `Init` and `Shutdown` *[compat]*

**`DoWinProcMessage(win, msg)`** (`0x00420870`): `0xA2BF8ACE` → the area is re-read from `+0x28` (slot 50), then
`DoPaint` and `win` slot 92 (invalidate), true; `0xA2BF8AD5` → `Init(win)`; `0xA2BF8AD6` → `Shutdown`; others false.

**`Init(win)`** (`0x004202F0`), once: initialised := 1; `+0x28` := win (counted); the area from its slot 50; the
screen size from the window manager's main window (slots 41, 42); a new camera with
`SetOrtho(0, W, 0, H, −400, 400)` (W, H the area's size); its projection matrix copied, transposed, to `+0x48`; its
local transform rotated by π (a float π widened, so the sine is about −8.7·10⁻⁸, not 0) in the plane of its first
and third axes, its "changed" bit (`0x04` of byte `+1`) set and its version byte (`+0`) incremented. True.

**`Shutdown`** (`0x00420540`), when initialised: clears the flag, `SetVisible(0)` (slot 6), releases the window, the
camera, the idle model, the second model and the playing model. True.

## 5. Drawing (`DoPaint`, `0x004209D0`) *[compat]*

W, H = the area's size, `p` = the pressed offset.

1. `PlaceModel(−W/2, −p, 0, 0.85 · H)` (slot 10; W/2 signed).
2. With a camera, a playing model and the 3D renderer (`0x00B43DD0`): the draw context (renderer slot 10) and its
   current camera (`+0x3C`) are taken; renderer slot 58; the graphics layer (`0x00B43CA4`) slot 74 sets the viewport
   `(L, screenH − B, W − p, H)`; the camera's viewport is `(W − p, H)`; graphics slots 17 `(1)`, 13 `(1.0)` and 11
   `(0x1000)` *[inferred: depth on, depth clear value, clear depth]*; the context gets its default render state, the
   face camera and depth offset 0; the model draws itself (its slot 12 `(context, 0)`); the saved camera is put back;
   graphics slot 75; renderer slot 59.

The pressed offset therefore nudges the head and narrows its viewport while an advisor button is held
(advisor-window.md, `0x287259F8`).

## 6. Quirks

- `Init` rotates the camera with a float π, leaving a tiny non-zero sine in the transform.
- The DollyLlama cheat only takes effect when the models are next loaded (a mood or visibility change).
- `SetMood` remaps the moods (0 → 3, 1 → 2, 3 → 0, 5 → 1; 2 and 4 unchanged) before indexing the advisor's model
  list, so moods 1 and 2 share a model and none maps to 4 except 4 itself.
- `DoMessage` answers true for every message, not only `0x86AD10EE`.

## 7. Function inventory

Owner: `0x004202F0` `Init`, `0x00420540` `Shutdown`, `0x004205D0` `LoadModels`, `0x00420870` `DoWinProcMessage`,
`0x00420900` `Animate`, `0x004209D0` `DoPaint`. Described, other owners: `0x00420030`, `0x00420100`, `0x004200A0`,
`0x00420130`, `0x00420140`, `0x00420220` (counted assignment), `0x00420B70`, `0x00420C00`, `0x00420C20`,
`0x00683EA0`.

## 8. The situation face (`cSC4SituationUIImageWinProc`) *[compat]*

A subclass (owner of 2 functions, 152 bytes; added 2026-10-24, re-gzsmallwin7) that animates on its own clock instead
of the tick manager:
- **`Init(win)`** (`0x0054A700`): when not initialised, `+0x28` := win (counted), then the base `Init`.
- **`DoPaint`** (`0x0054A660`): the timer at `+0xF8` gives the elapsed time; when it is at least 25 (the float at
  `0x00A9B754`), the timer stops, `Animate(elapsed)` (slot 13) runs, the base `DoPaint` draws and the timer restarts;
  otherwise only the base `DoPaint` draws.

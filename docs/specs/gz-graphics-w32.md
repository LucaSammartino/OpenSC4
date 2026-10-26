# The Windows graphic system `cGZGraphicSystemW32` and the screen buffer's Init (behaviour spec)

Scope: `cGZGraphicSystemW32`, the Windows subclass of the GZ graphic system (gz-graphics.md §5), and the one function
of `cGZScreenBuffer` that tools/coverage.py assigns, its `Init`. The base graphic system, its driver interface and
the canvas are gz-graphics.md; the screen-buffer colour type is span-render-4444.md §1.2; the creation through
`cGZGraphicWin32COMDirector` is com-directors-misc.md.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the disassembly; the vtables and the DirectDraw IID were read in
the PE. Written 2026-10-25 (re-gzw32-8). Offsets are of the **complete object** unless stated; a method reached
through an adjusted interface subtracts that interface's offset first (gz-graphics.md §5.1).

---

## 1. Identity *[compat]*

Constructor `0x0098B590` (`0x180` bytes, made by the Win32 graphic COM director): the base constructor
(`0x0098FDC0`), then the tables below and `+0x178` (the window handle) := 0. The deleting destructor (`0x0098B230`,
also through the `+0x04` thunk `0x0098B0A0`) reinstalls them, runs the base destructor and frees on flag bit 0.

| Offset | Table | Notes |
|---|---|---|
| `+0x00` | `0x00ADAB84` | primary; slot 0 QueryInterface `0x0098AD80` |
| `+0x04` | `0x00ADAB6C` | the system-service face (thunks `this − 4`) |
| `+0x1C` | `0x00ADAA58` | the graphics interface, IID `0x0073283C` (§2) |
| `+0x20` | `0x00ADA9F8` | the driver interface, IID `0xC47B747C` (gz-graphics.md §5.1) |
| `+0x24`, `+0x28`, `+0x2C` | `0x00ADA9C8`, `0x00ADA9B8`, `0x00ADA9A8` | the other base interfaces |
| `+0x170` | `0x00ADA958` | a helper face (§3) |
| `+0x174` | `0x00ADA93C` | the Windows interface, IID `0x5AD24C36` (§3) |

Every face's slots 0–2 are thunks (`0x0098B070…0x0098B220`) that subtract the face offset and jump to the shared
QueryInterface / AddRef (`0x0098ADC0`) / Release (`0x0098ADD0`); AddRef and Release are the system service's shared
counters. **QueryInterface**: IID `0x5AD24C36` → `this + 0x174` with an AddRef (primary slot 1), true (a null receiver
writes null and then calls through null *[quirk]*); any other IID → the base `QueryInterface` (`0x0098DEF0`).

## 2. The graphics-interface overrides (`+0x1C` face) *[compat]*

These fill slots the base leaves pure (gz-graphics.md §5.1). Offsets in this section are of the complete object.

| Slot | Address | Behaviour |
|---|---|---|
| 10 | `0x0098ADE0` | `GetScreenSize(out)`: `out := (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN), BITSPIXEL × PLANES)` of the screen DC (`GetDC(0)`, `GetDeviceCaps` 12 and 14, `ReleaseDC`); returns `out` |
| 18 | `0x0098AE40` | increments the word `+0x64` |
| 24 | `0x0098AE50` | `GetVideoMemory(·, wantFree)`: load `DDRAW.DLL`, `DirectDrawCreateEx(null, &dd, IID_IDirectDraw7, null)`, `dd->GetAvailableVidMem({caps 0x10005000}, &total, &free)`, release, unload; return `free` when the byte argument is set, else `total`. The first argument is not read. Any failure returns `0x1000000` (16 MB) |
| 33 | `0x0098AF00` | `GetCursorClientPosition(&x, &y)`: when the window `+0x178` is minimized (`IsIconic`), half the width and half the height of its normal-position rectangle (`GetWindowPlacement`, arithmetic shift); otherwise `GetCursorPos` converted by `ScreenToClient`. True |
| 66 | `0x0098B2A0` | **OnCanvasInit**, §2.1 |
| 67 | `0x0098B440` | **OnCanvasShutdown**, §2.1 |
| 68 | `0x0098AFD0` | slot 67, then tail-call slot 66 (re-initialise) |

### 2.1 OnCanvasInit and OnCanvasShutdown

**OnCanvasInit**: the canvas (`+0x108`) answers `0xE46397DB` (its window interface w; none: return) and the framework
its secondary face (`0x23EA70A1`, gz-framework-core.md §2.1; none: release w, return). When `w`'s slot 4 gives a
window handle: `+0x178 :=` it and the framework face's slot 6 receives it. When the byte `+0xA2` is set: clear it and
call the primary slot 13 with 1; otherwise clear the global byte `0x00B17ADD`. With a driver (`+0x20` slot 3): its slot
64 fills a record r; `+0xA0 := r+0x2E`, `+0xA1 := r+0x2D`, `+0xE0…+0xEC := r+0x10…r+0x1C`, and the depth d := `r+0x0C`
(32 without a driver). When the byte `+0x41` is set, the screen buffer (`+0x100`, its `+0x4C` interface) gets
`Init(width 0x00B17AE4, height 0x00B17AE8, flag 0x00B17ADD, type, bits)` with type 7 for `d ≤ 16`, else 9, and bits
from the type (8/16/32, `0x00449500`) — span-render-4444.md §1.2. Then `+0x34 := 0` and both references are released.

**OnCanvasShutdown**: `+0x34 := 2`; the framework face's slot 6 with null; the primary slot 13 with 0; the draw system
(class `0xEB2EA3CB`, IID `0x4B2EA3B3`, framework slot 5) gets its slot 9 when present.

## 3. The helper and Windows faces *[compat]*

`+0x170` face: slot 3 (`0x0098AD40`) returns the graphics interface (`+0x1C`; null for a null receiver); slot 6
(`0x0098B1D0`) returns the byte `+0xFC`; slots 14 and 15 (`0x0098B1E0`, `0x0098B1F0`) forward to the driver
interface's `SetDefaultDriverClassID` and `SetDriverSettingsIniFilePath`.

`+0x174` face (IID `0x5AD24C36`), with n := its slot 3 (`0x0098B600`, the word `+0x7C`, the driver count):

- slot 4 (`0x0098AFF0`) `(i, &string)`: for `i < n` and a driver d := `GetGDriverByIndex(i)` (driver interface slot 5),
  the string's slot 4 receives d's slot 77 (its name); true. Otherwise false.
- slot 5 (`0x0098B040`, six words): true when `i < n` and `GetGDriverByIndex(i)` is non-null.
- slot 6 (`0x0098B4F0`) `(i, m)`: for `i < n`, d non-null and `m <` d's slot 63 (its mode count):
  `g :=` the graphics interface's slot 14; `SetGDriver(null)`; the rectangle `+0x54…+0x60 := (0, 0, 1024, 768)`;
  `SetGDriver(d)`; return g's slot 5. `m` is only range-checked *[quirk]*. Otherwise false.

## 4. `cGZScreenBuffer::Init(width, height, flag, type, bits)` (`0x00991EC0`, on the `+0x4C` face, pop 20) *[compat]*

Offsets in this section are relative to the `+0x4C` face (the buffer base is at `−0x4C`).

1. Already initialised (`+0x04`): true. The base already initialised (`−0x44`): false. Width or height 0: false.
2. Hold the graphic system (`RZ::GS()`, `+0x28`) and its 3D device (`0xC47B747C`, `+0x2C`) when missing.
3. The rectangle `+0x18…+0x24 := (0, 0, width, height)`, copied to `+0x08` and to the base's rectangle (`−0x38`); the
   base colour pair (`−0x40`, `−0x3C`) := (type, bits); the base's slot 45.
4. The 4 × 4 auxiliary buffer `+0x30`, when missing: the graphic system's slot 3 creates it; its slot 3
   `Init(4, 4, type, bits)` — on failure it is released and cleared — then `Lock(0x8001)` (slot 6), on success
   `Fill(+0x34, 0)` (slot 18) and `Unlock(0x8001)` (slot 7).
5. Initialised := 1 (both `−0x44` and `+0x04`). The `flag` argument is not read.
6. With the draw system (`0x4B2EA3B3`): register a new `cGZDrawContextRenderToSimGLFactory` (`0xC` bytes, its slot 3);
   create class `0xAB300B20` as `0xAB300B2B` (a) and the draw context `0x0AE6320E` as `0x2AE63219` (b) through GZCOM;
   a's slot 7 with `(b, the buffer)`; b's slots 22 and 23 with 1; `+0x38 :=` a's slot 3 (held); release a and b.
7. Return true.

## 5. Conformance tests

1. `GetVideoMemory` without `DDRAW.DLL` returns `0x1000000` for both arguments.
2. A minimized window whose normal rectangle is (100, 50)–(900, 650) gives the cursor position (400, 300).
3. Windows-face slot 6 with a valid driver and mode sets the rectangle to 1024 × 768 even when the mode index names
   another mode.
4. Screen buffer `Init(0, 600, …)` returns false and allocates nothing; a second successful call returns true at once.

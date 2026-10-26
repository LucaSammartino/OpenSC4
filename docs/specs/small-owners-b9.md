# Small owners: texture setup, the 3D grid element, the limit timer and its calibrator (behaviour spec)

Scope: four small owners that tools/coverage.py assigns separately and no spec closes: `nsTexUtil` (image →
driver texture), `SC43DGridElement` (the renderer's quad-grid record), `cSC4LimitTimerCalibrator` and `LimitTimer`
(the CPU-cycle time-slice timer of simulator.md §5.5). Three owners that are already specified in full get their
coverage rows in the same lane: `tControllerTime` (automata.md §3.1), `LimitTimer` (simulator.md §5.5, pathfinder.md,
aura.md; restated in §4 below) and `cGZWinCustom` (widget-custom.md).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every body below was read in the disassembly. Written 2026-10-25 (re-smallown8).

## 1. `nsTexUtil` *[compat]*

Free functions used by the views that draw a bitmap as a 3D texture (dispatch.md, dispatch-vehicle-view.md,
network-occupants.md, render3d.md). "The driver" is a `cIGZGDriver` (usually the global `0x00B43CA4`); its slot 36
deletes textures, slot 60 is `CreateTexture(format, width, height, levels, flags)` and slot 61 is
`LoadTextureLevel(name, level, x, y, w, h, format, type, rowLength, pixels)` (renderer-driver.md §9.3–§9.4).
Buffer slots are the compiled `cIGZBuffer` ones: 3 `Init`, 6 `Lock`, 7 `Unlock`, 9 `Width`, 10 `Height`, 12 the
area (`cRZRect*`), 16 bits per pixel, 18 `Fill(color, null)`, 29 `Blt(src, srcRect, dstRect, clip)`, 34 the bits,
35 the stride (map-color-buffer.md, callout-box.md).

- **`GetNearestPowerOfTwo(n)`** (`0x006046B0`): the smallest power of two ≥ `n`, and 1 for `n ≤ 1`.
- **`ReleaseTexture(driver, name)`** (`0x00604690`, cdecl): with a non-null driver, driver slot 36 with `(1, &name)`
  (delete one texture); a null driver does nothing.
- **`SetupTexture(image, driver, &name, alpha, rect, &outW, &outH)`** (`0x006046D0`, cdecl, returns AL):
  1. `image.Lock(0x8040)`; if it fails, return **true** with nothing written *[quirk]*.
  2. `bits`, `w := Width()`, `h := Height()`, `bpp := BitsPerPixel()`; internal format `f := alpha ? 4 : 1`. With a
     rectangle `{l, t, r, b}`: `w := r − l`, `h := b − t`.
  3. `size := max(GetNearestPowerOfTwo(w), GetNearestPowerOfTwo(h))` (the loop is inlined).
  4. **Direct path**, when `w = size`, `h = size` and `bpp = 32`: `outW` and `outH` (each when non-null) := `size`;
     `name := driver.CreateTexture(f, size, size, 1 level, flags 2)`; the pixels start at `bits` or, with a
     rectangle, at `bits + t × stride + 4l`; `driver.LoadTextureLevel(name, 0, 0, 0, w, h, format 3, type 1,
     rowLength = the image's full width, pixels)` (source format 0, 32-bit 8-8-8-8). Unlock (`0x8040`), return
     **true**. Neither driver result is tested.
  5. **Resize path** otherwise: get the graphic system (`RZ::GS()`, `0x00913C1A`) and make a new buffer with its
     slot 3; if either fails, unlock and return false. `Init(size, size, colour type 9, 32 bits)`; on failure release
     the buffer, unlock and return false. If the new buffer locks (`0x8010`): clear it (`Fill(0, null)`), copy the
     image into its top-left corner (`Blt(image, image.area, image.area, null)`) and unlock it. Then, whether or
     not it locked, recurse: `SetupTexture(new buffer, driver, &name, alpha, null, &outW, &outH)` (which takes the
     direct path), release the new buffer, unlock the image and return the recursive result.

  So a 100 × 60 image becomes a 128 × 128 texture whose top-left 100 × 60 texels hold the image and the rest is
  colour 0, and `outW = outH = 128`. In the resize path a rectangle only sets the size: the copy takes the whole
  image from its origin *[quirk]*.

## 2. `SC43DGridElement` *[compat]*

The record that wraps one static model in the renderer's quad grid (render3d.md §5). Layout (0x34 bytes, from a
fixed-size page pool):

| Offset | Field |
|---|---|
| `+0x00` | vtable `0x00ABAAE0` (base `0x00ABAA38`) |
| `+0x04` | `u16` reference count |
| `+0x06` | `u16` flags (render3d.md §5) |
| `+0x08…+0x14` | the container rectangle (four `int32`) |
| `+0x18` | the model |
| `+0x1C…+0x30` | the box: six floats |

- **Constructor** (`0x007C3D50`, called by `cSC43DRender` at `0x007C5426` and `0x007CA2A7`): vtable; count := 0; the box
  := `(1, 1, 1, 0, 0, 0)` (it first copies the float at `0x00B4E1F8` into all six and then overwrites them, so
  that load has no effect). The rectangle, flags and model are left to the caller.
- **Deleting destructor** (`0x007C3DB0`, vtable slot): set the base vtable; with bit 0 of the flag argument, give
  the record back to its page (`this & ~0xFFF`): push it on the page's free list (`+8`) and decrement the page's
  count (`+0xC`); a count of 0 returns the page to the pool `0x00B0DC30` (`0x0090D031`), a count of `0x4000003A`
  moves a page that was full back to the available list (`0x0090CFF9`). This is the pool pattern of audio.md §4.2
  with 4 KB pages.
- **Base deleting destructor** (`0x007C1850`): set the base vtable; with bit 0, `0x005F4230(this)`, which pushes a
  non-null block on the free list headed by `0x00B0A934` (`*block := head; head := block`).

## 3. `cSC4LimitTimerCalibrator::Tick` (`0x005D9370`) *[compat]*

The calibrator is the simulator's member at `+0x80`; the simulator's `OnTick` calls `Tick` first (`0x00703E9E`) and
then publishes `CyclesToMillisecs` (`0x00B0A840`) := the calibrator's `+0x00` and `MillisecsToCycles`
(`0x00B0A83C`) := 1 / that (simulator.md §5.5). Fields: `+0x00` `f32 calib`; `+0x08` `u64` last TSC; `+0x10`
`u64` last wall-clock count; `+0x18` `u64` interval (one second of counts, simulator.md §5.5); `+0x20` `f32`
milliseconds per wall-clock count. The fields are set outside this body.

1. `q := cRZTimer::GetTimerCycle()` (`0x0088FF19`, `QueryPerformanceCounter`), then `CPUID` (serialising) and
   `c := RDTSC`.
2. If `q − q_last > interval` (unsigned 64-bit, strictly greater):
   - `dc := c − c_last` as a signed 64-bit integer;
   - if `dc > 0`: `calib := f32(calib × 0.9f + (q − q_last) × (msPerCount / dc) × 0.1f)` (x87, rounded once
     at the store; constants `0x00A88218`, `0x00A8C950`);
   - `q_last := q`, `c_last := c` (also when `dc ≤ 0`).
3. Otherwise nothing changes.

## 4. `LimitTimer` *[compat]*

A one-field object (`u64` deadline in TSC cycles):

- **`Start(int32 ms)`** (`0x0068BF50`, pops 4): `deadline := RDTSC + _ftol2(ms × MillisecsToCycles)` (the float at
  `0x00B0A83C`; the product is x87, truncated toward zero).
- **`Expired()`** (`0x0068BF80`): `RDTSC`; true exactly when `deadline − now` is negative as a signed 64-bit value
  (the deadline has passed); at the deadline itself it is still false.

## 5. Conformance tests

1. `SetupTexture` of a 64 × 64, 32-bit image with no rectangle makes one 64 × 64 texture and loads it directly;
   a 64 × 64, 16-bit image takes the resize path (same size, converted through a 32-bit buffer).
2. `SetupTexture` of an image that cannot be locked returns true and leaves `name` unchanged.
3. With `calib = 1.0e-6`, `msPerCount = 1.0e-4`, a sample of `q − q_last = 10,000,001` counts over
   `dc = 1.0e9` cycles gives `calib ≈ 0.9e-6 + 0.1 × 1.0e-6 = 1.0e-6`.
4. After `LimitTimer.Start(0)`, `Expired()` is true as soon as the TSC has moved past the start value, and false
   while it still equals it.

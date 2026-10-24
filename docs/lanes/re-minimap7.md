| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-minimap7: the city minimap window `cSC4WinMiniMap`, new spec minimap.md (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-audioprefs7) | **ready** (docs only). minimap.md §1–§13: identity, the three interfaces and lifetime, layout, city attach and the scale shift, the palettes and the ramp interpolation, the terrain/zone map (dirty blocks, height colour, shading, water blend, zones, resampling), networks, the rotated blits, painting (dither, frustum, marker), input, messages, flags and colours, quirks, ten conformance cases; questions MMW1–MMW4; owners claimed: `cSC4WinMiniMap*`, `fn:GZWinIsVisibleRecursive`, `fn:GZCOM_cSC4WinMiniMapCLSID`: specified +10,489 bytes (+41 functions) | MMW1–MMW4 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 41 claimed functions were disassembled, plus the constructor `0x007A8920`, the shared step helper `0x007A65D0`,
`nSC4UI::InterpolateRampToPalette` and the network filter. The "Mini map" exemplar was read from `SimCity_1.dat` with
`tools/dbpf.py`. Two packed-lane blends were checked by emulating the instructions on random inputs (0 mismatches).
Written on 2026-10-23. No upstream lane covers this owner.

### What changed

- New docs/specs/minimap.md.
- Read on the bytes:
  - the dirty-block bitset;
  - the height-to-palette index `64 + floor(0.2 × (h − sea))`;
  - the lighting: the normal's y scaled 0.4, the light at 63.75 × (±0.577, 0.577, ±0.577) by rotation, brightness
    `|d| + d + 127.75`;
  - the water lerp, the zone overwrite and the five resamplers;
  - the network bit pairs and the network pixel `0xFFCFCFC5`;
  - the rotation steps and the over-blend with the overlay;
  - the 4×4 dither matrix and the `0xF1F1F1` scale;
  - the frustum through `PickTerrainAlways` and the blink colours `0xFFC00000`/`0xFFFF4040`;
  - the click mapping, the message table and the tick period 1000.
- Shipped data: the terrain and water ramps.
- docs/specs/questions/minimap.md: MMW1–MMW4 (new prefix MMW); QUESTIONS.md regenerated.

### Quirks found

- `SetArea`'s `OnCityInit` is a no-op while attached.
- The scale comes from the x cell count only.
- The network bits survive a city shutdown.
- `InvalidateTZMapRect` clamps only the upper bounds.
- The zone overwrite test is always true.
- Network pixels are single when upscaled.
- The 4444 copy can exceed the surface for a non-square window.
- The click and frustum scales differ.

### Coverage

- tools/coverage-map.tsv: `cSC4WinMiniMap*`, `fn:GZWinIsVisibleRecursive` and `fn:GZCOM_cSC4WinMiniMapCLSID` →
  minimap.md, before the `cSC4Win*` catch-all.
- Specified bytes 4,445,219 → 4,455,708 (+10,489, 79.6% → 79.8%, `cov_local.py`, on `5c0ff782` + the five lanes
  before).

### Not covered

- The constructor's owner (`cSC4ScriptedWindowCOMDirector`).
- `nSC4UI` (only its ramp function is described).
- The GZ surface and draw-context interfaces (MMW2, MMW4).
- The marker's subject (MMW3).

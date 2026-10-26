| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzregion7: the region screen's scroll window and clouds (`cSC4WinRegionScroll`, `cSC4WinRegionClouds`), region-screen.md §12 (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzsmallwin7) | **ready** (docs only). Right-drag, glide and edge scrolling with their messages; cloud creation, drift and drawing; owners claimed: specified +4,503 bytes (+19 functions). Also corrects advisor-face.md §3 (the RNG upper bound is exclusive) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 19 functions were read in the decompile; both `GZPaint` bodies, `SetCloudArea`, the clouds' `Init` and the
region screen's call sites (`0x007AB9DB`, `0x007B1EFF`–`0x007B1F9F`) were followed on the instructions; the constants
`0x00A867A4` (0.001), `0x00AB7E10` (128), `0x00AB3AD0` (5.0), `0x00AB8B20` (2⁻¹⁴), `0x00AB8B28` (0.003),
`0x00A9422C` (256) and the cursor table `0x00AB8F2C` were read. Written on 2026-10-24.

### What changed

- region-screen.md §12 (§12.1 scroll window, §12.2 clouds).
- advisor-face.md §3 and §6: `RandomSint32RangeUniform(1, 3)` gives 1 or 2 (foundations.md), and a target of 1 can
  never be reached, so the second animation stops for good once 1 is drawn.

### Quirks found

- Clouds are only retired at the right edge; a left wind would keep them for ever.
- A released drag coasts to rest in 0.2 s; the glide stops at the first sign change of either axis.
- The advisor face's second animation stalls with probability ½ per cycle (advisor-face.md).

### Coverage

- tools/coverage-map.tsv: new rows `cSC4WinRegionScroll` and `cSC4WinRegionClouds` → region-screen.md, before
  `cSC4WinRegion*`.
- Specified bytes 4,778,252 → 4,782,755 (+4,503, 85.6%, `cov_local.py`).

### Not covered

- The 3D device render-state slots, cited by number; who sets the scroll window's extra velocity `+0xFC`/`+0x100`.

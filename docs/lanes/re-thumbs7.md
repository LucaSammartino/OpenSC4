| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-thumbs7: the region-view thumbnail functions (`fn:SC4WriteCityRegionViewThumbnail`, `fn:SC4ReadCityRegionViewThumbnails`, `fn:SC4RenderRegionBorderMasks`), regional-city-records.md §6.6 (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `14e0416c`, after re-rzdraw7b) | **ready** (docs only). The renderer's five helpers; the reader and the border-mask renderer were already described in §6.1–§6.2; owners claimed: specified +7,043 bytes (+9 functions) | RR1, RR2 stay open |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
The five uncited helpers were read in the decompile and the capture call `0x005F9510` on the instructions; their
callers in the renderer (`0x005DDF02`, `0x005DE3F9`, `0x005DE592`, …) were located. Written on 2026-10-24.

### What changed

- regional-city-records.md §6.6.

### Quirks found

- The opaque-bounds pass treats any colour byte above 3 as part of the city and forces its alpha to opaque.
- The four-capture average drops the alpha byte to 0.

### Coverage

- tools/coverage-map.tsv: three new rows → regional-city-records.md.
- Specified bytes 4,835,810 → 4,842,853 (+7,043, 86.7%, `cov_local.py`).

### Not covered

- What the capture interface `0xE69BFE2A` does with its four arguments.

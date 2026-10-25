| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-autorec7: the auto streams and auto records, auto-records.md; rows for three functions other specs describe (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-sllib7) | **ready** (docs only). The versioned stream and record helpers; `SC4CreateSavePathForCity` (region-model.md §7.4) and the two thumbnail-set writers (regional-city-records.md §6) claimed; specified +4,013 bytes (+23 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 19 functions of the auto owners were read in the decompile and the shared close helper `0x005D9430` in the
disassembly. The other three owners are fully described by existing specs: `SC4CreateSavePathForCity`
(`0x005DC400`, `0x005DC790`) in region-model.md §7.4, `SC4WriteCityRegionViewThumbnails` (`0x005DDA90`) and
`SC4WriteDefaultCityRegionViewThumbnail` (`0x005DDDC0`) in regional-city-records.md §6. Written on 2026-10-24.

### What changed

- New docs/specs/auto-records.md (builds on flora.md §10.3, which already verified the stream helpers).

### Coverage

- tools/coverage-map.tsv: the four auto owners → auto-records.md; `fn:SC4WriteCityRegionViewThumbnails`,
  `fn:SC4WriteDefaultCityRegionViewThumbnail` → regional-city-records.md; `fn:SC4CreateSavePathForCity` →
  region-model.md.
- Specified bytes 4,876,214 → 4,880,227 (+4,013, 87.4%, `cov_local.py`).

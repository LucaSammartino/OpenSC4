| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-querytool4 | .claude/worktrees/re-querytool4 | in progress (round 2 of 3) | round 2: DescribePick / DoQueryDialog / tip builders (CT11, CT12) |

## Handoff

Goal (round 2): city-tools-input.md §8 add DescribePick 004D4140, DoQueryDialog 004D3C30, SetTip 004C5BA0, tip
builders (GetBuildingOccupantTipInfo 004D5E30, GetNetworkOccupantSummaryInfo 004CF8A0, GetTerrainSummaryInfo
004D6C70), SetHighlightState 004CACC0 (CT11); update §8.10 inventory and §6.
Done: round 1 (c660810f, fbe696b7, e3683c54): 19 fns / 5,907 B.
Done round 2: 6f7dc97d §8.11 (SetTip, SetHighlightState, 3 tip builders, Ratio 004C3B10, headlines 004D5C90; CT11 answered in text).
Next: DescribePick 004D4140 (decomp line 9599), DoQueryDialog 004D3C30 (9422) -> §8.12; then inventory §8.10, §6 tests, answer CT11.
Facts: REA has no app open (static fallback). In DoMessage `this`=+0x28 so [ESI+0x1C]=callout +0x44.

## Report

Goal: specify `cSC4ViewInputControlQuery` (215 functions, 76,564 bytes per `tools/coverage.py --owners` at main
`fd9beb23`) in docs/specs/city-tools-input.md.

What changed (spec only; no tools, engine or docs/abi edits):
- docs/specs/city-tools-input.md §8 (new): creation by the 3D view commands `0x6A935CF3` (mode 0, query) and
  `0x6A935CF4` (mode 1, route query), layout `+0x28..+0x9C`, slot overrides (vtable `0x00A90A88`), Init/Shutdown,
  hover (OnMouseMove/OnMouseExit), clicks (OnMouseDownL/UpL), DoMessage dispatch of six messages, pick filters,
  timer callbacks, exact covered/remainder inventory (§8.10). §6 tests 6-8, §7 coverage note, outline.
- Questions CT6-CT12 in docs/specs/questions/city-tools-input.md; QUESTIONS.md regenerated.

Measured: covered 19 functions / 5,907 bytes (listed in §8.10); remainder 196 functions / 70,657 bytes (CT12).
The owner is not claimed as specified (the coverage map is a tools file and was not touched).

Corrections to Ghidra found on the instructions: OnMouseDownL in mode 1 returns right after GetOccupantPaths
(+0x89 stays set until OnMouseUpL) and DescribePick runs only when modifier bit 0x01 is clear; the 0.0001 hover
epsilon is a float (`0x00A9097C`). REA was not used (export + decomp/ sufficed).

Commits: c660810f (lane file), fbe696b7 (spec §8 + questions), then this report.

Next: round 2 on DescribePick `0x004D4140`, DoQueryDialog `0x004D3C30`, SetTip `0x004C5BA0`, the tip builders
(GetBuildingOccupantTipInfo, GetNetworkOccupantSummaryInfo, GetTerrainSummaryInfo) and SetHighlightState; round 3
on the route code and the SetOrGet* fillers.

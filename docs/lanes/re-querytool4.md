| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-querytool4 | .claude/worktrees/re-querytool4 | ready (round 1 of 3): query tool core specified, owner not claimed | round 2: DescribePick / DoQueryDialog / tip builders (CT11, CT12) |

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

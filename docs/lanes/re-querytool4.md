| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-querytool4 | .claude/worktrees/re-querytool4 | parked (round 3 of 3, final): 32 / 215 functions specified, owner not claimed | new lane: 87 filler bodies, zone/neighbour dialogs, route code (CT12); callout slots (CT13) |

## Report (round 3, final, 2026-10-21)

What changed (spec only): city-tools-input.md §8.12 (new): `DescribePick` 0x004D4140 (pick with the plain
filter, lot-building substitution, dialog choice table: neighbour connection / property `0x2A499F85` query dialog
/ zone 11-12-14 dialog / UI sound `0xAA62C9D5`; network and cell debug read-outs with their exact formats),
`DoQueryDialog` 0x004D3C30 (UI script {0, `0x96A006B0`, property value}, coverage-ring categories swap with a
330 ms timer, refresh globals `0x00B21840..4C`, modal run, get / set passes, return = result 0), the control walk
(ID bit `0x1000`, index `ID & 0xFFF` < `0x7F`, filler(window, ID, get, occ), descend on false) and the exact
103-slot filler table at `0x00A92488` (93 distinct, 89 owned, 12,323 B) with two sample bodies. §8.11.4 gauge
services named (police slots 25/28/36, fire 10, pollution 54/55). §6 test 10; §8.10 inventory round 3.
CT12 updated to the round-3 remainder; CT13 narrowed to the callout slots (services part resolved).

Measured: covered 32 functions / 16,046 bytes of 215 / 76,564 (round 3 added 8 / 4,321); remainder
183 functions / 60,518 bytes. Not claimed (coverage map untouched). REA had no binary open: static fallback
(export asm + decomp/). Ghidra's DescribePick frame is broken; the spec follows the instructions.

Parked (no fourth round): the 87 other filler bodies and their typed workers, `DoZoneBasedQueryDialog`,
`DoNeighborConnectionQueryDialog`, the description writers, route code, refresh walk `0x004D3B40`,
`EnumFuncClearFilter`, `PlayOccupantQuerySound`. Open: CT12 (remainder), CT13 (callout slots 4/14 shapes).

Commits round 3: 43500a1f, d23f3f17, 984e6a26, then this report.

## Report (round 2, 2026-10-21)

What changed (spec only): city-tools-input.md §8.11 (new): GetTerrainSummaryInfo (zone 0..15 string table, water
cells skipped, headlines), GetNetworkOccupantSummaryInfo (12-row network type order table, pipe debug line),
GetBuildingOccupantTipInfo (group test-order table with icon/secondary/fraction per row, R/C/I wealth maps,
transport and utility rules, `Ratio` 0x004C3B10 with worked values), the tip display tail of DoMessage and SetTip
(icon decides callout vs cursor text), SetHighlightState. §8.7 tip list corrected (fixed titles, local presets,
debug line appended after `
`; round 1's "empty result hides the callout" was wrong: icon 0 does). §6 test 9;
§8.10 inventory updated. CT11 answered (MATCHES, moved to answered/), CT13 filed (names of gauge services/values).

Measured: covered 24 functions / 11,725 bytes of 215 / 76,564; remainder 191 / 64,839 (CT12). Not claimed.
Corrections to Ghidra on the instructions: 0x1402 fraction is b / a guarded by b > 0 (Ghidra shows /0.0);
the tip builder takes 5 args (RET 0x14), title first. REA had no app open: static fallback (export + decomp/).
Not done: DescribePick 0x004D4140 (3,118 B) and DoQueryDialog 0x004D3C30 (783 B), context budget.

Commits round 2: 8ee45fc0, 6f7dc97d, 6c790829, 98eb55bf, then this report.

## Report (round 1)

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

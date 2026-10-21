| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-querytool4 | .claude/worktrees/re-querytool4 | in progress (round 3 of 3, final) | §8.12 DescribePick/DoQueryDialog, CT12/CT13 |

## Handoff (round 3)

Goal: §8.12 DescribePick 0x004D4140 + DoQueryDialog 0x004D3C30; SetOrGet filler / route family tables (CT12);
gauge services B43D98/B43D78/B43D9C, callout slots 4/14 (CT13). Final round: PARK with exact remainder if not done.
Done: (round 3 start) nothing yet.
Next: read DescribePick asm/decomp, write §8.12.
Facts: owner rows in build-runs/qowner.tsv (215); covered 24 fns / 11,725 B before round 3. REA: static fallback.

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

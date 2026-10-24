| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-wingrid-wg9 (WG5–WG9 grid contracts) | Codex / re-wingrid-wg9 | ready; WG5–WG9 answered; doccheck/questions PASS; ABI unchanged | Coordinator: apply contracts to core-gzwingrid9 and reconcile its WG7–WG9 open entries |

## Report

Extended [win-grid](../specs/win-grid.md) §§5.14–5.16 and §6 with the five requested contracts,
19 additional static conformance vectors, and coverage sections for every formerly pending owner row.
Corrected the earlier SetCellFont assumption: the instructions confirm both the font store and conditional
layout-dirty write. Corrected the paint format's colour offsets and identified +0x2C0 as the header font.

Material differences from the provisional core are unlimited bounds accepting negative indices, mode-based
selection membership, relative scroll filling the viewport, column-over-row format precedence, sentinel-sensitive
defaults, font draw alignment flags and a cross-field vertical-position quirk, null editor allocation fault,
whole-row header using header height as width, cell-area rather than per-cell clipping extent, unchecked
per-cell colour index, sparse-sort packing and SortRow's old-row selection lookup.
The default comparator uses narrow NUL-terminated bytes and the current CRT CTYPE locale; the grid does not
choose a locale. No game-specific locale or rendering verdict is claimed.

Evidence: read-only main-checkout export queries and bounded reads of `decomp/cGZWinGrid.c` and
`decomp/_unnamed/region_009a0000.c`; numeric/width rules checked against instruction bytes, with addresses in
the spec. REA was deliberately unused because the session has no binary open. The core lane's report names
its Owner closure table under `engine/`; this lane honored the no-engine-read rule and used the report's
explicit open-section list instead.

Validation: `python tools/questions.py index` regenerated the index; `python tools/questions.py check`
and `python tools/doccheck.py check` PASS. `git diff --check` PASS after removing trailing blank lines.
Cargo/clippy/CTest: n/a (spec-only). Game/shadow: n/a, no runs requested. No ABI exports or proposed ABI rows.
Answered WG5–WG9 in `docs/specs/questions/answered/win-grid.md`; WG3/WG4 remain open.
WG7–WG9 originated in the separate core-gzwingrid9 worktree, so the coordinator must retain these answers
when reconciling that lane's appended open questions.

## Handoff
- Goal: answer WG5–WG9, with behavior and vectors, for core-gzwingrid9.
- Done: first dependency milestone 433131a75; all five answers, owner inventory and 19 vectors complete in final commit.
- Next: coordinator publishes detached HEAD to reserved branch and merges spec; engine lane implements the differences.
- Detached HEAD is intentional. Main export/decomp are read-only; no REA binary open.
- No game requests outstanding; no ABI changes; WG3/WG4 are outside this task.

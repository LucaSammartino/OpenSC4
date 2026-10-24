| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-map-view-qv18 (map-view QV2/3/14 closure) | Codex / `re-map-view-qv18` (RE lane; detached HEAD) | in progress: resumed QV3/QV14; merged qv17 `8f269ba2a`; QV2 `465ada3f5`; preceding doccheck passed; ABI unchanged; game n/a | Finish QV3 panels/groups and QV14 ring precision, then doccheck and final queue. |

## Report

Merged the requested `re-map-view-qv17` prerequisite at `8f269ba2a`, including its earlier dependency
and QV1/QV5/QV11/QV16 closures. The sole merge conflict was the generated questions index; running
`tools/questions.py index` resolved it. The reserved worktree remains detached as instructed.

Closed **QV2** in `docs/specs/map-view.md` §4.1 with behavior and conformance vectors 36–39:

- Three fresh, cell-sized volume/X/Z grids; tract size 1 means shift 0.
- Travel-to-network mapping, positive-capacity eligibility, unsigned 17-word density records,
  scalar bias/cap and final signed balance clamps, including duplicate retained travel types.
- Separate view-`0x54` ferry accumulation: normal/alternate counts, both compressed paths,
  wrapping unsigned volume, outgoing-direction balances and saturation after each point.
- Damaged-road sprites, separately collected marked cells, phase-16 directional animation,
  palette-entry overrides and the final standard unsigned-16 draw with overlay collection suppressed.

The original QV2 entry moved to `docs/specs/questions/answered/map-view.md` with a dated DIFFERS answer.
Regenerated `docs/specs/QUESTIONS.md`. No new question ID or split, ABI row or game request.
QV3/QV14 remain open unchanged. QV15 remains the separate predecessor question outside this task's IDs.

Validation: foreground `python tools/doccheck.py check` **passed** (`doccheck check: ok`), log
`build-runs/doccheck-re-map-view-qv18.log`; `git diff --check` passed. Arithmetic/width rules were
verified against instruction bytes and selected PE constants; address citations are in the spec.
The PE constant reader is local ignored evidence at `build-runs/read-map-constants.py`, not shipped code.
No REA call was attempted: the brief states there is no binary open. Cargo/clippy/ctest and game/shadow
runs are n/a for this documentation-only RE work. No native ring edge-pixel equivalence is claimed.

Stopped new investigation at measured context **97046/150000**, near the requested two-thirds checkpoint.
This is a passing partial-ready result: QV2 is complete; QV3/QV14 still need a successor.
No engine, plugin, capi, ABI-table or coverage-map edits were authored; predecessor changes are carried
only through the expressly requested merge.

## Handoff
- Goal remaining: finish QV3 and QV14 without reopening the committed predecessor/QV2 answers.
- Done: prerequisite merge `8f269ba2a`; QV2 behavior, vectors and answer `465ada3f5`; doccheck passed.
- Next exact step: query DoMessage 0x007A56E0, then read main decomp/cSC4WinMapView.c lines 3219–3900 in ranges of at most 80 lines.
- QV3: resolve panels 0x8A2871B1–0x8A2871C4 and 0xAA32BCD4/5, group memories behind 0x5005/0x5008/0x8002, toggles and command 0x6A935CF1.
- QV3 helpers: ShowSmallWindow/group selection may be in decomp/_unnamed/region_00790000.c or region_007a0000.c; query before reading.
- QV14: helper 0x0079E130 leaves CW inherited; predecessor found CRT 0x009F6C30 setting PC=53. Establish RC/PC inheritance and exact store schedule, then a threshold vector.
- QV14 first instruction page was inspected only; no conclusion about runtime precision or RC was reached.
- QV2 caution: +0x548/+0x948 are the two palette[255] entries, not factors; factors are +0x958/+0x95C. +0x141 suppresses marked-cell collection, not lot blocking.
- No ABI reservation, REA operation or game request outstanding. Source paths under C:/Users/tlori/FreeCity4; edits only in this worktree; never read engine/.
- Round 2: verified qv17 is already an ancestor of HEAD; do not merge again or reopen QV2.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-map-view-qv18-r2-manifest.json.

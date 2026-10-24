| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-map-view-qv18 (map-view QV2/3/14 closure) | Codex / `re-map-view-qv18` (RE lane; detached HEAD) | in progress: QV3 `ee9d2ef1b`; QV14 complete pending final gate/report; merged qv17 `8f269ba2a`; QV2 `465ada3f5`; doccheck passed before final RC vector; ABI unchanged; game n/a | Final gate, commit report and update ready queue. |

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
- Goal remaining: final gate/report/queue; QV2/QV3/QV14 behavior and answers are complete.
- Done: prerequisite merge `8f269ba2a`; QV2 behavior, vectors and answer `465ada3f5`; doccheck passed.
- Round 2 milestone: QV3 panels/groups committed `ee9d2ef1b`, §§1.2/3.2.1/3.3, vectors 40–44.
- QV14 discovery: PreFrameWorkInit 0x0045236C calls 0x005F85E0, which sets PC=24 through 0x008929E0.
- QV14 vectors calculated by build-runs/ring-precision-qv18.py: r=513/128, v=-1/2, pixel(0,4),
  u=-511/1024 gives weights 254/255/255 for PC24/53/64; u=-1/2+2^-25 gives 254/254/255.
- QV14 complete: §5.2 operation/store schedule and floor-lower/ceil-upper bounds; vectors 45–48.
- RC inherited too: bias conversion at w=1/4 gives weights 255/256/255/256 for nearest/down/up/zero.
- PC24 is framework setup, not immutable: expression parser 0x0067BA40 can leave PC53; Lua wrappers restore.
- Next exact step: run build-runs/gate-re-map-view-qv18.py, finalize Report/Handoff, enqueue actual HEAD.
- Bash q.py wrapper failed before doccheck (CreateFileMapping access denied); direct foreground helper passed.
- QV2 caution: +0x548/+0x948 are the two palette[255] entries, not factors; factors are +0x958/+0x95C. +0x141 suppresses marked-cell collection, not lot blocking.
- No ABI reservation, REA operation or game request outstanding. Source paths under C:/Users/tlori/FreeCity4; edits only in this worktree; never read engine/.
- Round 2: verified qv17 is already an ancestor of HEAD; do not merge again or reopen QV2.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-map-view-qv18-r2-manifest.json.

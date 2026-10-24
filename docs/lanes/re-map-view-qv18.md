| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-map-view-qv18 (map-view QV2/3/14 closure) | Codex / `re-map-view-qv18` (RE lane; detached HEAD) | ready: qv17 merge `8f269ba2a`; QV2 `465ada3f5`; QV3 `ee9d2ef1b`; QV14 `aa6200ce1`; doccheck passed; ABI unchanged; cargo/clippy/ctest and game/shadow n/a | Land the documentation; implement/regress the closed QV2/QV3/QV14 contracts. QV15 remains outside this task. |

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

Closed **QV3** at `ee9d2ef1b`, in §§1.2/3.2.1/3.3, with vectors 40–44:

- Exact child visibility matrix for small, standard large, desirability and traffic layouts,
  including controls whose flags are left alone, missing children and the legend-content flag.
- One-time first-show button setup, persistent group memories, subsequent show/hide and expand commands.
- Independent desirability/traffic/top-level requested-view memories, aliases, button exclusion and
  subgroup enable states; selection failures still update requested memory and legend.
- Normal/alternate traffic byte, Reset request precedence and visibility, same-view suppression,
  and registered Cancel dispatch with zero/zero arguments.

Closed **QV14** at `aa6200ce1`, in §5.2, with vectors 45–48:

- Every x87 operation and square root rounds at the inherited PC/RC; explicit binary32 stores
  are separately identified. A binary64-only evaluation is not equivalent for all pixels.
- CRT PC53 is superseded by framework PC24 at `0x0045236C` through `0x005F85E0` and `0x008929E0`.
  These routines preserve RC. Later expression parsing can leave PC53 and Lua wrappers can temporarily
  change it, so the contract explicitly inherits actual caller state instead of inventing one permanent mode.
- Lower candidate bounds are floor and upper bounds ceil, correcting the old reversed prose.
  The integer bias/store/equality rule also specifies caller-modified rounding modes.
- Calculated nearest-mode threshold weights are **254/255/255** and **254/254/255** for PC24/53/64.
  A widened-bound pixel has weight 112. The separate RC vector gives weights **255/256/255/256**
  for nearest/down/up/toward-zero; the blend accepts the full integer weight.

All three original entries now live in `docs/specs/questions/answered/map-view.md` with dated DIFFERS
answers. Regenerated `docs/specs/QUESTIONS.md`. **No new question or split** was needed: inherited
floating-point state is specified behavior, not an unresolved global-mode question. The pre-existing
unsafe geometric-input boundary is retained. QV15 is the sole remaining open map-view question and
is outside this task's IDs. No ABI rows or game requests were proposed.

Validation: foreground `python tools/doccheck.py check` **passed** (`doccheck check: ok`), log
`build-runs/doccheck-re-map-view-qv18-r2.log`; `git diff --check` passed. The first round-2 gate attempt
through q.py failed before doccheck started because Git Bash could not create its shared mapping
(Win32 error 5), log `build-runs/q/20261024-205707-28068.log`. The subsequent direct foreground
helper `build-runs/gate-re-map-view-qv18.py` ran doccheck successfully, including after the final RC vector.
The earlier round-1 passing gate is separately recorded in `build-runs/doccheck-re-map-view-qv18.log`.

Arithmetic/width rules were checked against instruction bytes and PE constants; addresses are cited
in the spec. Local ignored evidence `build-runs/ring-precision-qv18.py` uses exact rational binary
rounding and integer square-root rounding to calculate the precision vectors and asserts the bound/RC
cases. These are static, instruction-derived calculations, not native game measurements or screenshots.
No REA call was attempted: the brief states there is no binary open. Cargo/clippy/ctest and game/shadow
runs are n/a for this documentation-only RE work.

This round completes both remaining requested closures. It supersedes the previous passing partial
checkpoint at `6d8395af4`; no partial work remains for QV2/QV3/QV14.
No engine, plugin, capi, ABI-table or coverage-map edits were authored; predecessor changes are carried
only through the expressly requested merge.

Resume verification (round 3): recovered the complete committed result at `f9cfd7eea` and confirmed
`re-map-view-qv17` is already an ancestor of HEAD; no repeated merge was needed. Reviewed the three
answer entries, their behavior sections and conformance vectors 36–48. All requested IDs occur only
in the answered file; QV15 remains the sole open map-view question. Regenerated the questions index
and reran foreground doccheck and `git diff --check`: **passed**, log
`build-runs/doccheck-re-map-view-qv18-r3.log`. No new RE investigation or spec change was required.
The ready queue entry is refreshed to this resume's full actual HEAD after committing the report.

## Handoff
- Goal complete: QV2/QV3/QV14 answered without a new split; QV15 is outside this lane's requested scope.
- Done: qv17 merge `8f269ba2a`, QV2 `465ada3f5`, QV3 `ee9d2ef1b`, QV14 `aa6200ce1`.
- Next exact step: coordinator lands the ready queue entry; implementers consume the spec and vectors 36–48.
- Numeric caution: framework PC24 does not imply immutable thread state; ring helper inherits actual PC/RC.
- Measured/calculated weights and gate evidence are in the Report above; no game request is outstanding.
- Worktree stays detached. Queue tip must be the full actual HEAD; supervisor publishes the reserved branch.
- No ABI proposal, REA operation, engine edit or external artifact was made by this round.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-map-view-qv18-r3-manifest.json.

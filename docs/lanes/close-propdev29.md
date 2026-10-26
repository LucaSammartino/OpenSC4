| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| close-propdev29 prop developer (engine) | Codex / close-propdev29 / reserved branch close-propdev29 (detached HEAD) | blocked; code bb03b5530, 33ffc3b01; cargo 7903 passed, clippy 0 diagnostics, CTest 126/126; observer C export added in 66e7f077a; gen27 queued without result; questions none | 1. Restore shared queue progress and collect gen27. 2. Complete remaining outputs/method evidence. 3. Validate both required profiles exact with zero resyncs, then enqueue. |

- Acceptance: whole-owner verified credit NOT met; no fully certified method, no current-source native exactness result.
- PV18 core fix: registration forwards both calendar bytes; missing/short properties supply (0,0), including interval zero.
- ABI: calendar host extension accepts legacy prefix; 66e7f077a added `fc4_prop_developer_shadow_effect`; no ABI version bump. This round adds no exports.
- Capture fix: freeze insertion properties/state/class/type/position/power/trigger/date at the genuine native handler entry; replay at the recorded nested boundary, preserving pre-tag inputs.
- bb03b5530: compare creation C AddRef/holder/lookup-or-add/property-value/u32-write/Release spans, immediate scalar output, reference deltas and nested boundaries; reject undeclared primary/holder layouts.
- 33ffc3b01: compare slot-13 float arguments/time bytes via the independent prop core; compare every followed step's saved owner projection, including incomplete/failed effects.
- Focused CTest: propdev 5/5 passed; negative cases reject wrong/missing/reordered writes/holds, unreadable output, custom layouts and stale dispatch; full log `build-runs/fc4verify-ctest.log`.
- Full gate PASS on 33ffc3b01: cargo 7903/0 (`build-runs/qbuild-test.log`); clippy zero diagnostics (`build-runs/qbuild-clippy.log`).
- Native gate PASS: 43/43 edges (`build-runs/fc4verify-build.log`), CTest 126/126 (`build-runs/fc4verify-ctest.log`); owned Rust fmt 5 files, diff_lines=0.
- Required house shadows: NOT RUN on this source; exactness and runtime resync counts remain unmeasured.
- Gen27 diagnostic: `close-propdev29-r1-gen27-1`, simulator + propdev shadow, script `gen27-konradshohe-s1-default`, target 2028-01-28; NOT a full house-profile request.
- Shared request: MAIN `build-runs/game-queue/close-propdev29-r1-gen27-1.req`, created 2026-10-26 12:05:05 UTC, still queued after this round's qwait timeout and full gate; no verdict/log/ref-warning measurement.
- Queue wait exceeds docs/harness.md's timeout + 15 minutes: blocked. Collect MAIN `build-runs/game-results/close-propdev29-r1-gen27-1.verdict`; do not duplicate the outstanding request.
- Method evidence added: cumulative `propdev method 0x006F4E90 cSC4PropDeveloper::DoMessage (month) ...`; complete remains zero because effects are incomplete.
- Compared partly: DoMessage return on agent/night/insertion paths; scanner/records, selectors, removals, creation arguments/results/tag spans; night CRT/state; dated/timer/chance/state/time-byte/RNG effects.
- No owner method is fully certified. Other owner interface/lifecycle/persistence methods lack full call/output comparison; UpdateTract/CreateProp and registration/night helper effects also remain partial.
- Remaining monthly failures still deliberately include every successful creation and Day with dated props. The existing `created ... (not compared)` and `full-operations-exact=0` claims were not weakened.
- Missing outputs: ordered model/frame calls and queued notification lifetimes, registration list/map holds and clock effects, member/wealth final RNG; other owner interface/lifecycle/persistence methods remain unverified.
- Next: inspect the pending diagnostic, fix actual divergences per spec, then run the house shadows and full gen27 profile; require every propdev line exact, zero resyncs, clean verdict and ref_warnings=0 before enqueue (kind engine).
- Questions: none. WIP merge-queue entry only; not enqueued (kind engine), because exactness/completeness and required game checks remain unmet.

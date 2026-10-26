| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| close-propdev29 prop developer (engine) | Codex / close-propdev29 / reserved branch close-propdev29 (detached HEAD) | continue; code ee9e0430a, c996e6cfa, aeeecad9b; cargo 7895 passed, clippy 0 diagnostics, CTest 124/124; ABI exports unchanged; gen27 queued; questions none | 1. Collect pending gen27 verdict. 2. Complete PV18 effect comparisons. 3. Validate house shadows and gen27 exact with zero resyncs, then enqueue. |

- Round checkpoint: context 151871/200000 before finish; implementation is incomplete, with no verified credit claimed.
- PV18 core fix: registration forwards both calendar bytes; missing/short properties supply (0,0), including interval zero.
- Host extension: appended calendar callback; 276/552-byte current tables accept the legacy 272/544-byte prefix. No C ABI exports or ABI version changed.
- Capture fix: freeze insertion properties/state/class/type/position/power/trigger/date at the genuine native handler entry; replay at the recorded nested boundary, preserving pre-tag inputs.
- Tests cover calendar pairs, short properties, zero-interval membership, a genuinely shorter host allocation, changed native state, nested snapshots and missing-input refusal.
- Full gate PASS on aeeecad9b: cargo 7895/0 (`build-runs/qbuild-test.log`); clippy zero diagnostics (`build-runs/qbuild-clippy.log`).
- Native gate PASS: 45/45 build edges (`build-runs/fc4verify-build.log`), CTest 124/124 (`build-runs/fc4verify-ctest.log`); owned Rust fmt check passed for three files.
- Required house shadows: NOT RUN on this source; exactness and runtime resync counts remain unmeasured.
- Gen27 diagnostic: `close-propdev29-r1-gen27-1`, simulator + propdev shadow, script `gen27-konradshohe-s1-default`, target 2028-01-28; NOT a full house-profile request.
- Shared request: `C:/Users/tlori/FreeCity4/build-runs/game-queue/close-propdev29-r1-gen27-1.req`; still present at 2026-10-26 12:25 UTC after two qwait timeouts. No verdict/log yet; no ref-warning or exactness claim.
- Collect its result in MAIN `build-runs/game-results/` using qwait; do not submit a duplicate while it remains outstanding. The staged DLL will include the completed native gate.
- Method evidence added: cumulative `propdev method 0x006F4E90 cSC4PropDeveloper::DoMessage (month) ...`; complete remains zero because effects are incomplete.
- Compared partly: DoMessage return on agent/night/insertion paths; scanner, saved-state projections, ordered selector replay, removal identities, creation arguments/results and final requester-tag projection; night shuffle CRT/state.
- No owner method is fully certified. Other owner interface/lifecycle/persistence methods lack full call/output comparison; UpdateTract/CreateProp and registration/night helper effects also remain partial.
- Remaining monthly failures still deliberately include every successful creation and Day with dated props. The existing `created ... (not compared)` and `full-operations-exact=0` claims were not weakened.
- Next: implement PV18 semantic dated/timer/RNG and ordered model/post/lifetime traces, registration outputs and creation C/holder/tag/Release spans (spec 3.3.2-3.3.3, 4.4); compare every output before increasing complete.
- Next: inspect the pending diagnostic, fix actual divergences per spec, then run the house shadows and full gen27 profile; require every propdev line exact, zero resyncs, clean verdict and ref_warnings=0 before enqueue (kind engine).
- No spec questions filed; no merge-queue entry or enqueue, because this is a continuation checkpoint.

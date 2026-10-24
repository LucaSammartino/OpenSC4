| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-disasters-house11 (countable house disaster shadow) | Codex / `glue-disasters-house11` | in progress | Inspect shadow and harness; implement deterministic schedule; gate and measure house to 2032 |

## Handoff
- Goal: deterministic scripted disaster house shadow, monthly countable exact summaries through 2032.
- Worktree reused at reserved detached HEAD; no branch changes permitted.
- Done: added opt-in disasters_scripted fire/riot schedule and cumulative monthly dated comparison lines; native gate next.
- Implementation commit: 6345f0625. DS47 filed for observation contracts for short-form disasters; DS28 remains open.
- Intermediate gate TERMINAL PASS: qbuild all Rust 7119/0, clippy 0 diagnostics, native build 260/260 edges (349.7s), CTest 81/81 (876.8s). Request qbuild-20261024-121859-build-ctest, session 64395 completed. It reported tree changed; NOT final-source validation. Logs preserved under build-runs/intermediate-house11/.
- Bounded fixture committed in 21f74d387: remove leaders via occupant manager slot 21 after 200 riot samples; own holds through both ending ticks. A further diagnostic fixed the completion guard: an ended riot keeps ticking/dirty (section 6.7), so completion tests both layers' ongoing counts rather than dirty=false.
- Final-source all TERMINAL PASS on 45485ab28 (engine source eec9904dc): Rust 7119/0, clippy 0 diagnostics, native 52/52 edges in 4.3s, CTest 81/81 in 344.5s, native total 358.0s; NO tree-changed warning. Request qbuild-20261024-130822-build-ctest; session 9629 completed. Logs at build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log.
- Bounded correction implemented: after 200 riot samples, pin leaders with Keep into a separate owned set, RemoveOccupant(occ, false, 0), retain holds through original/mirror ending ticks, DropAll at EndCity. Native final-source gate still required after the intermediate request terminates.
- Prepared request: build-runs/house11.req.pending, label house11-fire-riot-01, not submitted yet. build-runs/wait-house11.py waits without qwait.
- No game request outstanding and no DLL/game measurement yet. build-runs/compare-house11.py compares dated mismatch counts against the explicitly named main build-runs/regress-train193-shadows.log.
- Next: task-required explicit qbuild verify now (collect its session to terminal); then submit prepared house request with train193 switches plus disaster keys, timeout 1800. Use one outstanding game request and unique label; no qwait. Do not edit engine source after the passing full gate unless a real diagnostic requires it.
- Monthly failures include numbered differences, refused ignition resyncs, unfollowable inputs, missing leaders, call-result differences and guarded-core failures.
- Schedule uses original manager start APIs and waits for natural lifecycle completion, with turbo held off then restored; no forced resume.
- Constraints: no tools/ reads or edits; propose regress profile change in report only. Native verify uses queued qbuild verify. No qwait for game requests.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-disasters-house11-r1-manifest.json.

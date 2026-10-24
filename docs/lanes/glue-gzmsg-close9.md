| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-gzmsg-close9: GZ manager verification | Codex; glue-gzmsg-close9 (detached HEAD) | WIP: gates pass; native manager unavailable, gzmsg 0/73 | Existing-manager fixture or complete capture contract required for verification; house profile still needed |

## Result

Verification did not close. Exact measured line: **`gzmsg-close9-r1-2032 0/73`** (`gzmsg 0/73`). All 73 samples report unavailable, captured=0, probes=0, mismatches=1, resyncs=0. No native manager method could be compared. A WIP queue entry holds this work because there is no measurable native manager state; no verified coverage credit is requested.

GZ12 now explicitly explains this result: shipped startup registers a class factory, not the optional manager service. Its absence cannot be repaired by constructing or installing a manager in a live city. GZ11 explicitly denies complete read-only observation. Neither answer supplies the facility needed to achieve the requested whole-manager verification.

## Changes

- Implementation commit: `6c10a0bc0`. The existing shadow already used the correct service/IID. Acquisition now explicitly documents null initialization, success-only QI ownership, borrowed framework lifetime, and no factory fallback. A manager hold spans its queue lookups; successful queue holds are released before the manager. Failed lookups do not authorize releasing a pointer.
- Native slot-9 ID-entry count joins the bounded diagnostic. Its immutable load value is compared independently; it is not inferred from the finite Rust registry, which cannot include hidden IDs. Individual lookups replay against the production Rust QueueManager2 seeded only with the load probe results. No reseeding or native mutations occur.
- Dated summary uses exactly the lowercase label `gzmsg shadow YYYY-MM-DD:` and explicit available/unavailable status. Numeric zero count fields during unavailable samples are placeholders, never an exact-empty result. Diagnostic remains default-off behind gzmsg_shadow.
- Added all eight spec §5.3 vectors, GZ-V15–V22: absent service preserves output; QI owning acquisition and class/service distinction; alias ID cardinality; hidden ID; held lookup after removal; equal bounded samples concealing mutation; ordinary posted-count boundary; endpoint observations concealing middle contents. These are portable host-controlled fixtures, not native city measurements.
- No C ABI exports changed. No ABI bump, generated ABI list, tools file, coverage table, or main merge/ref update.

## Validation

- Focused core tests: 20 passed, including all eight new vectors; 0 failures.
- Explicit owned-file qbuild fmt: PASS (four Rust files).
- Stable-source qbuild all: **6992 passed, 0 failed, 0 ignored; clippy 0 diagnostics; native build 253/253; CTest 79/79; PASS**. Native steps were handed to the coordinator queue by all as request `qbuild-20261024-074607-build-ctest`, collected to terminal PASS. No second native verify was launched. The tree-changed note reflects a lane-documentation commit during the check; engine source stayed unchanged.
- Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log; native verdict build-runs/build-results/qbuild-20261024-074607-build-ctest.verdict.
- [Run 463](../runs/gzmsg-close9-r1-2032-476.md): focused Konradshohe turbo diagnostic to **2032-01-01**, clean; foreign_exceptions=0, ref_warnings=0, watchdog_stalls=0, rtc=0, overruns=0; 28.797 real seconds, 40 s wall. DLL SHA256 `4bd22766e5411e819e4c7db5604e7c587ddf1a8a4be85f424f780650b63ca638`.
- This was **not the house-shadows profile**. The current ini settings were requested asynchronously because tools/regress.ps1 is forbidden to this lane; no response arrived. No profile was guessed or changed. No normal-speed repeat: acquisition already fails on load, before turbo, so there is no turbo-only comparison difference.
- Game evidence: build-runs/game-results/gzmsg-close9-r1-2032.{verdict,log}. One request, no outstanding request, no rerun. The custom notification wait missed the already-completed verdict; that valid verdict was retained and read directly, and the stale waiting operation was canceled and collected to terminal exit. No qwait was used. Initial wait failed because the results directory did not yet exist; helper corrected by creating the directory. q.py also failed at Bash startup (CreateFileMapping access denied); direct PowerShell cargo succeeded. These were helper failures, not engine/gate failures.

## Observation and coverage limits

When a manager exists, only GetQueue over the 259 documented diagnostic IDs (0–255, service, IID, UINT32_MAX), lookup Boolean/numeric identity, and GetQueueCount against its independent immutable load observation are compared. Count is unlocked; individual lookups are locked, so their combination is not atomic. Equal pointers/counts do not prove absence of intervening mutations or pointer reuse.

Manager AddQueue, CreateQueue, both RemoveQueue forms, RemoveAllQueues, destruction, QI/base forwarding and reference-count return traces remain uncompared. So do unprobed ID bindings, complete queue contents/order/search and blocking-signal behavior, server notification registrations, directed posts, pending changes and full post state. No full registry enumeration or mutation observer exists under GZ11. In this run even the bounded lookup/count methods were unmeasurable because no manager was acquired. No manager, queue, or server coverage-map rows can be promoted.

## Questions and next steps

No new spec questions: GZ11 and GZ12 are answered in framework-gz-services §3.1/§3.2; GZ8–GZ10 remain existing separate open questions. Unavailability differences are explained by GZ12, not suppressed as exact.

1. Coordinator decides whether to land the bounded diagnostic/tests independently; queue remains WIP under the instruction that nothing natively measurable stays WIP.
2. Supply an authorized existing-manager fixture/profile if bounded native lookup/count evidence is desired. Do not create/insert a service in the live city to manufacture success.
3. Full owner verification additionally requires an independently specified complete capture/replay facility; an unchanged finite probe cannot establish it.
4. Supply the current house-shadows ini profile for its required safety run. Only an actually exact eligible run can justify profile/coverage changes.

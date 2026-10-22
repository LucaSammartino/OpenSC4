| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-portdev-union-final4: AT24 aircraft lifetime holding ports union | Codex / `glue-portdev-union-final4` (engine; detached HEAD) | ready; native lifecycle corrected; cargo4714/0, clippy clean, ctest68/68; final paired reload/save/ordinary clean2033/ref0; fresh shadow26/26 exact0resync; additive C ABI export, no version bump | Coordinator joint train PASS before any ports promotion; final logical round3/3 exhausted. |

## Report

Corrected the common aircraft teardown timing that produced the same three warnings with and without ports. Source is only `automata.md` sections5.2.1-5.2.2 and the public `re-aircraft-ownedrefs4` report. No port simulation, tools, maps, defaults, audit macros, private refcount fields, or unrelated class replacement changed. No new questions; all unrelated automata questions remain.

### Root cause, observed before changing lifetime

Instrumentation commit `7e4d7a3b` preserved existing behavior. It records Read IID/output receiver/index, manager/class, caller phase, AddRef result, Release result, and the corresponding previous count inferred as result+1. It never takes a probe reference or inspects a private field.

Both diagnostic runs reached2033/ref3 on DLL`00213eb1` with the identical immutable fixture and supplied script:

| Run | Aircraft manager | Base plane receivers (Read order) | Airport keys (paired Read order) |
|---|---|---|---|
| pduf4-diag-ports-20261022-a | 04A6A9F0 | 03080814,03081814,03081014 | 031437A8,031436D8,03143670 |
| pduf4-diag-control-20261022-a | 04D5EA20 | 03070814,03071814,03071014 | 031337A8,031336D8,03133670 |

Each Read retained three base references plus three airport/plane pairs: nine acquired interfaces, with each plane appearing in both owners. Aircraft Shutdown released none. Later member destruction released base planes2->1, then each airport2->1 and map plane1->0 in stream order. All three warnings were those last map-plane releases in **destruction after Shutdown**, not the native initialized Shutdown event. The control establishes the same common failure independently of ports. Pointer identities and full logs are in `build-runs/game-results/<label>.log`; supplied earlier controls are under `build-runs/evidence-run4`.

### Implementation

- `c0dcb51a` moves initialized aircraft cleanup to Shutdown: ascending unsigned x86 airport-pointer order, airport then plane per pair, reset map and runway storage, then base plane Shutdown/release and clear the list. PreCityShutdown clears only the separate base list.
- Final aircraft manager Release invokes Shutdown before deleting storage. Repeated Shutdown has no payload pass. The aircraft deleting/storage destructor releases no payload references; the other managers retain their existing behavior.
- Added `fc4_automata_aircraft_lifecycle`, an additive C ABI callback bridge, to dispatch public plane methods at the owning core events. Operations cover pre-shutdown, initialized Shutdown, and an already-accepted airport removal; removal calls plane slot13(false), erases, then releases plane before airport. The SDK binding for slot13 is `Remove(false)`.
- Existing airport-message acceptance/filter wiring is unchanged. This is a bounded AT24 lifetime correction, not complete aircraft simulation, filter, unplace/camera, or unrelated automata implementation credit.
- Generic HostRelease remains normally audited. Only exact lifecycle release events, matched to the aircraft manager receiver, use the existing last-holder annotation justified by AT24. It still requires an acquired reference; no reference audit checks or macros changed. Read replacement/failure, other managers, and destruction retain ordinary auditing. No destructor-only annotation was added.
- `e9cbb658` adds seven independent counted-interface regression families using hand-encoded version5 input and separately controlled table/external owners. They test table disposal, base-only pre-shutdown, map-first Shutdown without pre-shutdown, external owner survival, reversed unsigned pointer keys, removal/miss, and repeated explicit/final-release-shaped Shutdown versus storage destruction. Real-game traces independently confirm final manager Release calls empty Shutdown after explicit Shutdown.

In the final no-save paired run, early native-phase Shutdown still sees external owners: map plane5->4, then base plane4->3. After explicit save, the measured corresponding counts are4->3 and3->2. Airports drop3->2 without save and2->1 after save. These are actual Release return values, not a guessed fixed warning count. The independent counted tests also cover the legitimate zero-count final owner and delayed external-owner destruction. Thus the timing/order correction is visible separately from audit classification.

### Verification

Baseline `qbuild all`: cargo4707/0, clippy0 diagnostics, ctest68/68; initial instrumentation overlapped its later build, so this baseline is not claimed as the stable final gate. Final stable `qbuild all` at code commit`e9cbb658`: **cargo4714/0, clippy0 diagnostics, CMake5/5, ctest68/68 PASS**. No code changed after this gate. Focused CAPI tests527/0 include the seven new families. Both touched Rust files pass the required wrapper fmt check; `git diff --check` passed.

Early quick iterations caught a C++ interface-name error and a test type alias error, both corrected before the final gate. Two incremental quick invocations were conservatively marked failed by the wrapper's Ninja restat edge parser (1/4, no compiler error); the final full build completed5/5 and passed normally. No tooling changes or verdict overrides.

Final DLL SHA256: `457179c94a045048736b14733811761c0188285f98474230cfab7cb61e4db08f`.

| Final-DLL run | Exercised case | Verdict |
|---|---|---|
| pduf4-final-ports-20261022-a | current39-key union+ports, supplied saved-port fixture/script | clean2033/ref0 |
| pduf4-final-control-20261022-a | identical DLL/fixture/script, ports disabled | clean2033/ref0 |
| pduf4-final-ordinary-20261022-a | ordinary city, candidate union+ports,2026 through2033 | clean2033/ref0 |
| pduf4-save-ports-20261022-a | same saved-port fixture, supplied script plus full save | clean2033/ref0; SaveCity ok1 |
| pduf4-save-control-20261022-a | identical save script/DLL/fixture, ports disabled | clean2033/ref0; SaveCity ok1 |
| pduf4-final-shadow-20261022-a | same fixture/script, ports original with shadow, current union | clean2033/ref0; airport13/13+seaport13/13 exact; resync0 |

Earlier corrected DLL`2fc4855f` paired runs `pduf4-fixed-ports-20261022-a` and `pduf4-fixed-control-20261022-a` also both reached2033/ref0. The final DLL pair repeats their result after diagnostic wording/formatting was settled. All requests have unique labels; only one was outstanding at a time. Completed verdicts were retained. There were no failed-start/harness-error retries. No turbo-only shadow difference occurred, so no normal-speed rerun was required.

The exact enabled union was obtained from the coordinator's specifically named public train84 reload log, without directory scans:39 optional existing keys, two enabled defaults (simulator,budget), plus ports. The control removes ports only. Final game results all identify the same DLL. The unchanged prior144/144 exact ports proof remains valid; this lane additionally measures a fresh saved-city26/26 exact0resync under the current union.

### Actual port path and fixture integrity

The input is the previously captured real replacement port-city save, loaded through the supported queue `fixture_city` path. Final replacement logs show airport and seaport Read each with ports1 before Init, port-property switching calls3 to state0, then calls3 to state1 and calls3 again for the no-transition case. The save run logs both developers Write returning1 with ports1 and `SaveCity(full)` returning ok1. The control also saves successfully. Every run exits through full city Shutdown/final Release.

Immutable fixture: `C:/Users/tlori/FreeCity4/build-runs/fixtures/glue-portdev-complete4/pdc4-replace-20261022-a.sc4`,1001692bytes, SHA256`38f234433b0c387393a7e25a971953c37b7127ea93c7d5d0a4cb2c789e78b0be`, verified before requests and after the suite. The supplied reload script remains SHA256`3886fc13c51f9a21308870ee17e0a71f42e394e19e3f1c47ff56e404f1127bdb`. The extra save scripts affect only isolated slot copies; the immutable source and Documents were untouched. Game files are not committed.

Detailed ignored artifacts: `build-runs/at24-evidence.json`, `game-results/<label>.verdict/.log`, `qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. Queue/wait helpers and copied enabled-key evidence are confined to this worktree's `build-runs`. q.py could not launch Bash (E_ACCESSDENIED), so bounded native PowerShell reads and quiet qbuild wrappers were used.

### Coordinator next step

Integrate this additive C ABI export under the coordinator's ABI batching policy (no lane version bump), then run the joint train gate/regression. Only the coordinator may promote ports after joint train PASS. No default, map, tools, or coverage promotion was performed here. This exhausts the final logical round3/3; no fourth round, subagents, restart, merge, push, shared-ref/config changes, native RE inputs, or external communications. Detached commits succeeded despite Git's denied packed-refs-lock diagnostic.

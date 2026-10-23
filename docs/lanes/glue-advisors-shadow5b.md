| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-advisors-shadow5b: advisor shadow AD19 | Codex / glue-advisors-shadow5b | in progress | Apply coordinator AD19 comparison rule, gate, then run through 2032. |

## Handoff

- Goal: exact record-only advisor shadow through 2032 under coordinator AD19 harness decision; no replacement factories.
- Prior tip 30c2cf74: AD17 adapter and date fixed; cargo 6536/0, clippy clean, CTest 69/69.
- Prior turbo strict 1/73 and normal strict 1/4: all differences precisely AD18; details below.
- Coordinator permits expected-record normalization of exactly documented Read fixups, counting fixups=N; never restore native flags.
- Next: archive AD19, update comparator/native fixture, qbuild all (includes queued verify), unique-label 2032 game.
- No request outstanding. Worktree intentionally detached; do not switch refs.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-advisors-shadow5b-r2-manifest.json.
- tools/ is forbidden: coordinator must append advisors_shadow to regress profile after exact run.

## Report (2026-10-23)

Implemented AD17's record adapter for CitySituationAdvice (`0x8BE3753B`) in the C ABI saved-object factory, preserving its subclass CLSID and reference identity while using the base advice serializer. The adapter is explicitly limited to records; it supplies no additional city-situation interface or live subclass behavior. No native replacement factories were registered. `advisors_shadow` remains default off and is enabled only through the existing ini key. C ABI exports and version did not change. Implementation: aa8ac388.

The load comparison now obtains the date from the city's simulator instead of the observer's not-yet-published date. The load sample correctly prints 2026-01-27 and compares exactly. Records include advisor lists/event queues and advice mood/priority values. MySim advisor records remain out of scope, with six identities retained in the system map, as in round 1.

AD18 requires Read to enable the persisted text-dirty bit. The shadow still compares every captured byte strictly. A separate diagnostic classifies a mismatch as a load transition only when the complete record matches after the sole expected `F OR 0x10` change, with stable references, version-4 base framing and successful Read/Write. It never restores or masks the actual output, changes the native object, or credits transition conformance as exact. Every remaining game difference passes this complete-record classification.

The native graph fixture now covers two distinct CitySituationAdvice records, subclass identity, a subject shared with the advisor event queue, already-dirty exact records, clean plain/city-situation strict mismatches and native reference ownership. No native Read is invoked. The ABI fixture covers plain/city-situation classes with flags `2`, `0x80000000`, `0x12` and `0x80000010`, verifying strict versus transition results and unchanged framing.

Validation on stable engine source: focused ABI fixture **1 passed / 0 failed**. Required `qbuild all`: **6536 passed / 0 failed / 0 ignored**, clippy **0 diagnostics**, final native build **46/46 edges**, CTest **69/69** including `advisors_shadow_test`; final native job `qbuild-20261023-221859-build-ctest`, **77.8 s** total. Logs: `build-runs/qbuild-{test,clippy}.log`, `build-runs/fc4verify-{build,ctest}.log`.

Queueing mistake retained for review: explicit `qbuild verify` was submitted before discovering that `all` also queues native verification. Both original operations reached terminal PASS. Explicit job `qbuild-20261023-221430-build-ctest`: build **220/220**, CTest **69/69**, **233.0 s** total; its changed-tree note reflects documentation commits only. The final all job had no changed-tree note. No further full check was launched. The fmt check exposed pre-existing weak-handle formatting in `advisors.rs`; only the new test's formatting was fixed. An initial focused-test attempt through q.py failed to start Git Bash (CreateFileMapping access denied); direct Cargo then passed. That failed wrapper evidence remains in `build-runs/q/20261023-221332-32004.log`.

Game: **adv-shadow5b-20261023-r1-2032 1/73** strict exact. [Run record](../runs/adv-shadow5b-20261023-r1-2032-330.md). Konradshohe turbo reached **2032-01-01**, verdict **clean**, `ref_warnings=0`; foreign exceptions/stalls/rtc/overruns all zero. **1479 differences, all 1479 precisely AD18 load transitions, zero other differences**, with 73 classification lines. Load: 858 records / 57833 bytes / 0 differences. Final month: 861 records / 57902 bytes / 22 differences. Records ranged 858–861; mismatches ranged 0–29.

Normal-speed confirmation: **adv-shadow5b-20261023-r2-normal 1/4** strict exact. [Run record](../runs/adv-shadow5b-20261023-r2-normal-331.md). Reached **2026-04-01**, verdict **clean**, `ref_warnings=0`, other problem counts zero, no turbo activation. **31 differences, all 31 precisely AD18 transitions, zero other differences**, four classification lines. The differences therefore also occur at normal speed. Both runs used DLL SHA256 `f4db142b37f7379725dc531249ecbed4ee0defd37ae7e99554a27aa7550ec80f`; full verdicts/logs are in `build-runs/game-results/<label>.{verdict,log}`. All build/game waits completed; none remains outstanding.

Filed **AD19** in `docs/specs/questions/advisors.md` (dd6caee6): whether a separate live-record import, independent of load Read and output restoration, has a specified contract permitting exact comparison of clean flags. Current literal implementation keeps the specified Read and every strict mismatch. AD18 explains all measured differences; strict verification credit is not claimed. Coverage/regress tooling was not edited. The coordinator should regenerate the questions index using its permitted tooling.

Next: obtain the AD19 contract or accept that this Read/Write shadow stays unverified for clean native advice. Do not normalize AD18 differences into exact credit. If a separate import is specified, implement it with focused fixtures, repeat the final gate and real-city run to 2032, then consider coverage and regress-profile credit. WIP queue entry holds this lane for review.

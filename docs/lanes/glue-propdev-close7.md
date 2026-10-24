| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-propdev-close7 (prop developer verification) | Codex / `glue-propdev-close7` | ready | Land measured work; route PV18 to RE; retain default-off shadow |

## Changes

Round 2 source commit `9600542f6` implements the section 3.3.1 live linked-list dated walk: each callback returns before the next link is read, null values are skipped, and no temporary occupant reference is taken. Removal copies the last node's value and unlinks that last node, as section 4.7 requires. Reentrant invalidation of the current node has no defined native outcome and is not credited as ordinary expiry. A focused test removes a future entry and appends another during the callback, proving the walk observes the live links.

The bounded `lightinghour 0..23` script action calls the genuine native `SetTimeOfDay(float hours)` producer (lighting.md sections 2.1/2.2, SDK slot 6), using PV15's borrowed renderer/lighting-manager path. It neither synthesizes dusk/dawn nor guesses a signature. Parser boundaries and integration action-list expectations are covered. `clockmode` remains available from round 1.

Null creation-service results are eligible for comparison when their complete arguments/result stream matches: section 4.4 performs no post-return registration/tag writes for null. Successful creation remains ineligible under PV18. The created-output counter now excludes failed attempts. Monthly summaries report eligible operations separately from matching projections; `full-operations-exact=0` remains explicit. `propdev_shadow` stays default-off.

Round 1 commits add the PV16 scoped native message observer, same-thread CRT capture and independent ordered-list/callback-return comparison, plus requester-tag output projections. The cumulative lane adds C ABI export `fc4_prop_developer_shadow_crt`; this round adds no export and never changes the ABI version.

## Validation

- Focused live dated-walk test: 1/1 passed; final owned-file fmt passed. The initial q.py invocation failed before cargo because Git Bash's mapping was denied; direct PowerShell cargo supplied the focused result.
- Stable final source full gate `qbuild-20261024-070413-build-ctest`: Rust **6940 passed, 0 failed**, clippy **0 diagnostics**, native **61/61 edges**, CTest **78/78**, PASS. Session 3069 terminal; native verification 221.6s. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. No source changed during or after this gate.
- Normal-speed `gpc7-r2-light-normal-2032-a` reached **2032-01-01**, clean, **151.563s**, DLL **0a9789a1**; foreign exceptions, stalls, rtc, overruns and **ref_warnings=0**. Session 63239 terminal; no operation remains outstanding. Wait used `build-runs/wait-file.ps1`, never qwait. Run record [436](../runs/gpc7-r2-light-normal-2032-a-436.md).
- Exact clean/total: **`propdev 0/72`**; harness text: `propdev 72 lines, 0 with every mismatch count 0`. Resyncs **0**.
- Eligible operation projections: **42537 eligible, 42537 matched** out of **45465 deliveries**; **2928 incomplete**. Monthly mismatch total is exactly **2928**, all incompleteness diagnostics covered by PV18; no additional observed divergence.
- PV16 genuinely exercised: **6 night callbacks, 6 exact, 975 CRT draws**. Native producer logs show midnight `changed=1 night=1` and noon transitions. Selector events **24200/24200**, complete replay streams **45465/45465**, invalid streams **0**, final requester-tag projections **562/562**. Those tag projections do not certify the complete creation effects.

## Limits and next steps

PV18 remains open in `docs/specs/questions/prop-developer.md`; no new question this round. Still ineligible: successful creation registration writes and hold/holder/tag callback ordering; dated timer/state/date advancement, separate occupant RNG, queued notification lifetime/order, and model writes/releases. Do not replay an opaque native-effect transcript as independent verification, read unnamed date-record fields, or tick a native occupant twice.

Land this measured work as READY, then route PV18 to an RE lane. After its contract is specified, implement independent delegated-effect observation before promoting those operations or claiming N/N. The explicit `enable=simulator` profile is not a separately established full house profile. Neither coverage-map nor regress tooling was read or edited.

## Earlier measured evidence

Round 1 normal-speed requests remain valid: `gpc7-pv16-normal-2032-a` (record 434, DLL 489fe5e3, 146.891s) and `gpc7-clock-normal-2032-b` (record 435, DLL 80210e7d, 132.593s), both clean to 2032 with ref_warnings/rtc/foreign exceptions zero and `propdev 0/72`. They had zero night callbacks and do not exercise PV16. Their selector streams were respectively 24197/24197 and 23581/23581, with 45465/45465 complete replays each. The second had 588/588 requester-tag projections and 2966 incomplete operations.

Round 1's clockmode action first failed CTest 77/78 on a stale expected action list (`qbuild-20261024-064433-build-ctest`); failed evidence remains in `build-runs/fc4verify-ctest-clock-golden-failed.log`. Corrected-source `qbuild-20261024-065026-build-ctest` passed Rust 6939/0, clippy clean, CTest 78/78. Round 2's gate above supersedes it.

Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-glue-propdev-close7-r2-manifest.json`. Detached HEAD is intentional; the supervisor publishes the commits to the reserved lane branch.

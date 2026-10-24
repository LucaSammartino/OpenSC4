| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-landvalue-capture7: LV8 acceptance and measured fallback | Codex / glue-landvalue-capture7 (reserved detached HEAD) | ready for measured fallback only; native capture disabled, LV9 host dependency | Provide host exclusion, install actual capture, then three canonical house repeats exact/ref0. |

## Report

**Native per-Step capture is not installed, and the recurring flake is not fixed.** This is the task-authorized READY fallback for tested acceptance fixtures and measured output, not readiness of the requested native interceptor. LV8 closes the native question by explicitly rejecting lifecycle-only exclusion. Section 6.1.4 requires an identified host mechanism and does not supply an existing game installation API. None has been identified or installed here; that integration dependency is LV9. Capture remains disabled and every comparison retains input_capture=pass-end-sample; native_step_capture=0.

### Changes and prerequisite reconciliation

- Ran the requested merge of glue-landvalue-step6 and re-landvalue-lv8 first. Git recorded merge77280d42d but its octopus helper failed with sandbox CreateFileMapping error5, omitting the side changes. Recovered the actual merge-tree contents with apply_patch in b0bf6ce0e, preserving both parents, resolving the additive CMake conflict, and removing the now-answered open LV8 entry while retaining step6 LV2 evidence. No unrelated weather/propdev or propmgr branch was merged.
- Added engine/conformance/tests/landvalue_capture_contract_test.cpp and an additive CTest target at the end of engine/CMakeLists.txt. The six deterministic reference-host fixtures implement spec16.26-31: lifecycle-only rejection; separate new/loaded startup coverage and actual pre-call D; fetched-destination admission; retained notification targets after unsubscribe; shutdown during an outstanding after-copy and numerical-address reuse; failed validation and foreign-route ownership.
- The fixture explicitly models admission before destination fetch, separate retained-target leases, owned developer map copies and once-only unchanged-result forwarding. It does not implement synchronization in the plugin, inspect native memory, modify routes, certify an installation or satisfy LV2/LV7 runtime provenance.
- Updated FC4LandValueShadow.cpp startup wording: LV8 is closed and lifecycle ordering is not dispatch exclusion; a host exclusion mechanism is still absent. Source milestone9ddac837c.
- Filed LV9 and preserved measured differences under LV2/LV7 in docs/specs/questions/landvalue.md. Coordinator should regenerate the question index; this engine lane may only edit the question file and cannot run tools/ scripts.
- No algorithm, native signature, sampling, map selection, comparison, golden output, default switch, C ABI export/version, generated ABI entry or coverage/regress file was changed.

### Stable-source gate

qbuild.py --wt this-worktree all: **PASS**, operation79019 collected to its actual terminal result.

- Workspace tests: **6917 passed / 0 failed / 0 ignored**, summed by qbuild; build-runs/qbuild-test.log.
- Clippy -D warnings: **0 diagnostics**; build-runs/qbuild-clippy.log.
- Native build and full CTest delegated by that same operation to coordinator queue qbuild-20261024-052425-build-ctest. No direct MSVC configure, duplicate verify or redundant quick build.
- Native build **244/244 PASS**,105.2s; build-runs/fc4verify-build.log.
- CTest **76/76 PASS**,159.6s; build-runs/fc4verify-ctest.log. landvalue_progress_test and landvalue_capture_contract_test both passed.
- Verifier noted the tree changed during its run: documentation milestones were committed while it waited. No engine source changed after9ddac837c; no source/input diagnostic justified repeating the full gate.
- git diff --check passed. Git repeatedly reported a denied packed-refs.lock during commits, but detached HEAD commits succeeded and were verified; no shared refs/config were changed.

DLL SHA256: ABAD0198BAA9D88E545B6FC76BAD3EF57350671A28F928F2A14C95FB62C18B59.

### Real-game measurements

Canonical house settings were requested as a clean-room copy at build-runs/house-profile.req; none was supplied. tools/regress.ps1 was not read. These are explicitly isolated fallback measurements and do not establish equivalence to the house profile. Exactly one request was outstanding at a time. Event-based waits collected both existing verdicts to terminal results without qwait or polling loops.

- [Run410](../runs/lvcapture7-20261024-isolated-t1-410.md): **lvcapture7-20261024-isolated-t1 71/73**. Turbo, clean/ref_warnings=0, through2032-01-02, real37.110s, slot4, wall52s. Pass1 at2026-01-28:383 total differences(max15), other fields exact, live developer map (LV7). Pass3 at2026-03-06:1 slope difference(max0.013541221618652344), all other fields exact, previous developer map (LV2).
- [Run411](../runs/lvcapture7-20261024-isolated-normal-411.md): **lvcapture7-20261024-isolated-normal 72/73**. Required normal-speed counterpart, clean/ref_warnings=0, through2032-01-01, real110.828s, slot3, wall122s. Pass1 at2026-01-27:383 total differences(max15), other fields exact, live developer map (LV7). No later differences. Startup failure is not confined to turbo; absence of the slope difference here does not establish a global turbo-only cause.

Both runs have rtc=0, foreign_exceptions=0, watchdog_stalls=0, overruns=0, incomplete0 and resynced0. All differences are retained under numbered LV2/LV7; native_step_capture=0 throughout. Requests, logs and verdicts remain under build-runs/game-queue/done and game-results. The exact-count script agrees with both harness summaries. No every-line-exact run, native input-fidelity, deterministic-fix or new coverage credit is claimed. The requested three exact72/72 canonical house turbo repeats **were not achieved**.

### Questions and next steps

LV9 tracks missing host provision, separately from the answered native LV8 question. LV2/LV7 retain the measured input/startup gaps. The next implementation needs a demonstrated route-wide pre-fetch admission/exclusion mechanism, publication before the first initialized delivery on both city paths, world/input lifetime through before/native/after copies, shutdown ordering, ownership-safe restoration and callback retirement. Reference-host fixtures alone are not that mechanism.

After that host exists, install/version-gate actual capture and wire the owned per-Step inputs, including state7 actual developer maps and loaded startup provenance. Gate that source and require three unique canonical house2032 turbo runs with every landvalue line exact and ref_warnings=0. Until then, capture remains off. No build, game or wait operation is outstanding; this READY entry is restricted to the measured fallback scope above.

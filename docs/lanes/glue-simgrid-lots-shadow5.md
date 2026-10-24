| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-simgrid-lots-shadow5: sim grid record shadow | Codex, `.claude/worktrees/glue-simgrid-lots-shadow5` (reserved detached HEAD) | blocked: existing native request unserviced; no DLL | Coordinator service native request; resume full gate and SG3 game run through 2032. |

## Report

Round 3 resumed at detached HEAD `5da9bbedb`. No implementation changed: native verification is externally blocked.
At **2026-10-23 23:52:24 +01:00**, the original request
`build-runs/build-queue/qbuild-20261023-232536-build-ctest.req` remained unserviced (written 23:25:36).
`engine/build-x86/FreeCity4.dll` does not exist and `build-runs/build-results/` is empty.

The round-3 single blocking waiter (session 7552) reached its actual terminal result after nine minutes:
`timeout: verdict not available`, exit 1. It watched
`build-runs/build-results/qbuild-20261023-232536-build-ctest.verdict`.
There are **no live local waiters and no game requests outstanding**. The original native request is preserved.
No duplicate verify or full gate was launched while it was pending. The final `qbuild all` remains required
after the pending request is serviced; prior-round Rust results below remain the only measured gate stages.

No game run occurred, so there is no run record or `<label> <clean>/<total>` evidence. Exactness through 2032,
native CTest, and `ref_warnings=0` remain unmeasured. A READY queue entry would be unsupported; the queue stays wip.
No new questions or C ABI changes. Lot records remain outside round-3 scope under existing LR12.

Next: coordinator service the existing native request; collect its real result and fix any diagnostic;
run the final full gate on stable source. Submit unique label `sg3-record-2032-r3-20261023a` through 2032
using `build-runs/sg3-record-2032-r3-20261023a.request-template`, then collect it with the existing blocking
PowerShell waiter (no qwait). Diagnose any shadow differences and rerun with a new label. Record exact/total,
clean verdict and ref warnings. The clean-room rule forbids this lane from opening any `tools/` file, so the
coordinator must add `simgrid_shadow = 1` to the regression shadows profile only after measured exactness.

## Prior report

Merged `re-simgrid-sg3` as `62809139`, then implemented `simgrid_shadow` in `6458df71`.
The new key defaults off and is separate from the existing `simgrids_shadow` fixture switch.
At PostCityInit and each month it reacquires the 38 fixed SG3 roles, walks the current growth-developer
map in key order, and observes four grids per developer (86 roles for the shipped twelve developers).
No native grid is initialized, resized, modified or retained across observations.

Each grid's own Write is read into an isolated Rust SimGrid; private bridge operation 14 returns its
instance ID, complete public geometry and every typed tract in x-major order. These bytes are compared
with the native getters. Signed values and float bits are preserved. Missing owners/grids, failed map
access, type/geometry mismatches and read/capture failures are explicitly reported and cannot count as
exact. Residential triple-output references are acquired/released; borrowed single-grid pointers are
never released. This is record-read verification over SG3, not an algorithm differential or a claim
about owner-private/map-view grids. No C ABI export or version changed.

Validation on stable implementation source:

- Focused `fc4-capi simgrids_verify::tests`: **4/4 passed**, including two new full-read tests for signed
  bits, non-default loaded geometry, float NaN/negative-zero bits and truncated records.
- Explicit-file fmt passed. `qbuild all` workspace tests: **6537 passed, 0 failed, 0 ignored**;
  clippy **exit=0, diagnostics=0**. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`.
- Native verification remains **unmeasured**. The single queued request is
  `build-runs/build-queue/qbuild-20261023-232536-build-ctest.req` (queued 23:25:36; still unstarted
  at 23:38:50). Its local waiter 69384 was terminated with exit=1 after over 13 minutes waiting;
  the request remains for the coordinator. Do not enqueue a duplicate; collect its actual result.
- Sequencing error preserved separately: `all` also enqueued native request
  `qbuild-20261023-233052-build-ctest` while explicit verify was pending. That duplicate `.req` was
  removed before pickup; its wrapper 65757 was terminated with exit=1. Therefore `all` as a whole
  is **incomplete**, despite successful Rust/clippy stages. No native success is inferred.
- Initial relative-path fmt failed to locate the file; absolute-path fmt passed. q.py failed on Bash
  CreateFileMapping permissions; the focused cargo check ran directly in PowerShell. Neither is a
  source/compiler failure. Git commits completed despite packed-refs.lock permission warnings.

No game request has been submitted, no game labels are outstanding, and no `<label> <clean>/<total>`
line or run record exists. Exactness through 2032 and `ref_warnings=0` are unmeasured. Do not credit
coverage or add the key to a regression profile yet. The prepared, unsubmitted request template is
`build-runs/sg3-record-2032-r2-20261023a.request-template`; `wait-sg3-verdict.ps1` supplies a single
blocking wait without qwait, accepting an already completed unique-label verdict.

**LR12** is filed in [questions/lot-records.md](../specs/questions/lot-records.md): define whether the
full record walk means master-list lots or every section 1 class, and specify complete discovery of
off-list lots/business units/occupants. The master lot list alone does not establish the latter scope.
`lotrec_shadow` is deferred without guessing native signatures or claiming partial coverage. The
coordinator must regenerate the questions index; this lane did not read or run tools/ scripts.

Next: service the existing native request, diagnose any actual compiler/CTest failure, then submit
the unique SG3 game request through 2032. Preserve any existing verdict. Record exact/total lines,
clean verdict and ref warnings before changing this entry to ready for simgrid alone. Resolve LR12
before implementing lotrec_shadow. The current merge-queue entry is wip and must not land as verified.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| tools-refill-validation4 (evidence preflight and heartbeat recovery) | `Codex` / `tools-refill-validation4` (tooling lane) | ready: source b42c1c80 complete; coordinator full pytest 180 passed / 1 POSIX-only skip on clean tip 476aa600; earlier missing-pytest failure retained below. ABI unchanged; cargo/clippy/ctest/game n/a; questions none. | Land through the merge train; rerun combined full tools suite after integration. |

## Changes

Commits: b00b3819 (first evidence fix), b42c1c80 (heartbeat recovery, final regression tests and contract note).

`tools/lane_evidence.py` implements the actual optional evidence-list contract. Absent evidence and `[]`
remain valid; scalar/dict/numeric/null containers and non-string/empty items fail explicitly. Relative paths
use the supplied repository root, not cwd or the spec directory; absolute paths, spaces, Unicode and legal
punctuation remain supported. Real stat/read operations reject missing files, directories, invalid native
paths and unreadable sources. OSError/ValueError diagnostics name the spec, field/index, value and reason.
There is no prose heuristic or external-path confinement, and copy basenames/overwrite order are unchanged.

`lane_refill --queue` validates each requested spec before its single queue write: any bad spec leaves queue
and coordinator-state bytes unchanged and consumes zero rounds. Direct launch validates before locking,
retirement/state writes or setup, rechecks after existing guards before worktree/SDK setup, and wraps source
read failures at copy time. Deleting evidence after enqueue is caught at launch. Scheduling/capacity,
dependencies, locks, provider controls and guard denials retain their authority.

`lane_heartbeat.CriticalReader` centralizes coordinator/queue/train reads for snapshot/report, initial train
discovery and event refresh. It retries OSError and invalid/midwrite JSON (including wrong top-level container)
at most five attempts with 0.25 s sleeps, within 2 s and the caller wait deadline. Recovery checks STOP and
the freshly read current provider pause between attempts. It never supplies empty/stale healthy state.
Persistent failures identify the critical path and last exception and return exit 1; caller deadline exhaustion
returns exit 2 with the diagnostic. Valid no-current-train JSON remains usage exit 3. A failed snapshot writes
neither a new heartbeat/log nor a replacement of previous artifacts. Existing train/file events still precede
backlog checks; freshness and file-event behavior pass the original tests. OS reads already in progress cannot
be interrupted by this retry budget.

Writer inspection: laneguard.write_json uses a temporary file and os.replace, retrying PermissionError;
merge-train.save_state writes state.json directly. This supports handling both read-sharing and midwrite
failures, but does not establish the cause of the observed live PermissionError. No live-state probe was used.

## Measured validation

New tests are standard-library unittest cases, also discoverable by pytest. All commands ran in this worktree:

- `python -m unittest discover -s tools/tests -p test_lane_evidence.py -v`: **Ran 10 tests, OK (skipped=1)**,
  the skip is POSIX-only filename punctuation/whitespace. Windows syntax, actual CLI enqueue, the incident
  string, all-or-nothing multi-spec writes, root/cwd resolution and early direct launch were exercised.
- `python -m unittest discover -s tools/tests -p test_lane_heartbeat_reads.py -v`: **Ran 10 tests, OK**.
  Subcases cover all three critical files, permission/parse recovery, real partial JSON completion, persistent
  errors in discovery/event/snapshot/report, preserved/absent heartbeat artifacts, attempts/time/deadline bounds,
  provider switching/STOP during recovery, valid no-current train, and nonzero report CLI failure. No real sleeps.
- `python build-runs/check-existing-heartbeat.py`: **Ran 16 tests, OK**. This ignored stdlib adapter executes
  the unchanged existing heartbeat test functions with temporary paths and scoped mock patches; it covers
  dependency/backlog, train event priorities, freshness, file events, STOP/provider and timeout behavior.
- `python -m unittest -v test_lane_backends.Launcher.test_provider_switch_retains_round_cap test_lane_backends.Launcher.test_parked_waiting_disk_duplicate_and_glue_caps`
  (PYTHONPATH=tools/tests): **Ran 2 tests, OK**.
- `python -m compileall -q` on changed Python source/tests and `git diff --check`: no errors.
- Required final stable-source command, **after b42c1c80 was committed**: `python -m pytest -q tools/tests`
  printed **C:\Program Files\Python312\python.exe: No module named pytest**, exit 1, no tests collected.
  The earlier focused pytest command failed for the same reason. An isolated ignored-directory dependency
  attempt found no available pytest distribution and installed nothing. No full-suite pass is claimed.

Captured logs are in this worktree's ignored `build-runs/`: evidence-unittest.log, heartbeat-unittest.log,
heartbeat-existing.log, capacity-unittest.log, focused.log and tools-suite.log. Native PowerShell was used:
q.py could not start Bash (E_ACCESSDENIED). No game/build/Rust gates were needed or run; no ABI changes.

## Limits and disposition

Source is complete and committed; the merge entry is held as `wip` because the mandated full pytest suite
cannot run in this interpreter. The coordinator must run that command with pytest available, investigate any
failures, and only then mark the entry ready. Windows behavior was measured; POSIX-specific filename behavior
is covered by a platform-conditional test but was not executed here. No live run4 queue/state, engine, RE,
shared guard internals, provider availability or main branch was changed. No game runs or questions filed.

## Coordinator validation, 2026-10-22 22:15 BST

The missing pytest module was handled explicitly with an ignored coordinator virtual environment at
`build-runs/pytest-venv`, using existing system/user packages. In this clean worktree at tip `476aa600`,
`C:/Users/tlori/FreeCity4/build-runs/pytest-venv/Scripts/python.exe -m pytest -q tools/tests`
completed with **180 passed, 1 skipped in 134.79 s**, exit 0. The complete log is retained in this worktree's
`build-runs/coordinator-full-tools-2220.log`. The skipped filename test requires POSIX. Independent coordinator
review of all eight changed files found no blocking defect; `git diff --check` passed. Source is unchanged
from the tested tip; this follow-up records validation only. The held entry can now be ready. Heartbeat exit 1
is an explicit critical-read failure requiring inspection; it must not be treated as a healthy snapshot or pause.

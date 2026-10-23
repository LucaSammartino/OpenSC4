| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| tools-process-inspection4: read-only process snapshot | Codex, `tools-process-inspection4` | ready | Coordinator review and normal tools merge queue |

## Changes

Added supported `.claude/lane-bin/process_snapshot.py`, concise usage in
`docs/ops/process-snapshot.md`, and 29 behavioral tests in
`tools/tests/test_process_snapshot.py`. Initial substantive commit: `91499337`;
conservative scope refinements and validation: `3a7cd298`.

The helper uses only installed psutil, takes an explicit target and optional recorded
PID/creation-time/argv-digest identities, and writes timestamped JSON to its own
worktree's build-runs. Summary includes normalized target and UTC/local capture time.
Same-user target/build candidates are separated into target-related, foreign and
unknown groups. Readable identities, cwd/exe, redacted actual argv, parent/child
relationships and target file handles carry provenance. No environment or full
unrelated command lines are emitted. Foreignness requires readable fields and only
absolute argv operands; opaque arguments remain UNKNOWN without positive evidence.

Creation-time changes, is_running failure, stale recorded creation time, changed argv,
denied reads, disappeared processes, empty/failed/truncated inventories and partial
reads are explicit. Diagnostics cap at 100 with omitted counts. File handles are
references, not proof of writer intent or exclusive ownership. Every snapshot is
non-authoritative and exits 3; valid readable captures distinguish OBSERVED from
NO_TARGET_OBSERVED, while failures/unknown candidates use UNKNOWN. Input errors exit 2.
There is no permission to launch or control processes implied by any observation.

## Validation

Commands ran from this worktree through q.py's direct argument-list transport:

- `python -m unittest discover -s tools/tests -p test_process_snapshot.py -v`:
  printed `Ran 29 tests in 0.147s`, `OK`.
  Log: `build-runs/q/20261023-023538-28764.log`.
- `python -m unittest discover -s tools/tests`:
  printed `Ran 194 tests in 25.974s`, `OK (skipped=1)`.
  Log: `build-runs/q/20261023-023547-25604.log`.
- `python .claude/lane-bin/process_snapshot.py --target C:/Users/tlori/FreeCity4/.claude/worktrees/tools-process-inspection4`:
  printed `process snapshot: UNKNOWN counts={} authoritative=false`, exit 3.
  Capture UTC `2026-10-23T01:33:41.903344+00:00`, local
  `2026-10-23T02:33:41.903344+01:00`.
  JSON: `build-runs/process-snapshots/20261023T013341.903344Z-30124.json`;
  log: `build-runs/q/20261023-023341-30528.log`.
  Actual error: `psutil unavailable for this Python interpreter; no fallback or installation attempted`.
  Thus **no live process inventory was obtained**; counts are unavailable, not zero.
  `py --list-paths` showed only `C:/Program Files/Python312/python.exe`.

Mocks assert attributable target/no-target captures, access denial, disappearing
processes, changed PID creation times, argv mismatch, partial read races, foreign
paths and output handles, redaction, unreadable ownership/scope, bounded inventory
and diagnostics, parent creation order, malformed inputs, unsupported platforms and
missing dependency/provider diagnostics. Live attributable inspection remains
unverified in this worker environment; the explicit dependency-error path was
demonstrated for real. No package/configuration changes were made.

q.py's single-string Bash transport was denied by the Bash service; direct argument
transport succeeded. Git emitted a packed-refs.lock permission diagnostic during
commits but created the detached HEAD commits normally; status was clean afterward.
Cargo, clippy, ctest and game runs: n/a for this tools-only change. C ABI unchanged.

## Questions and next steps

No questions filed. Coordinator can review and land via the normal tools queue.
For a live attributable capture, use an already configured interpreter with psutil;
this lane does not install dependencies or repair worker permissions. No WMI,
process management, service/cache RPC, other-worktree source reads, engine changes,
hook/policy changes, merge, rebase, push or shared state/default/coverage edits.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| tools-capacity-supervision4: durable capacity and dispatcher ownership | Codex, `tools-capacity-supervision4` (reserved detached worktree) | WIP: implementation complete; 50 focused checks pass; full pytest unavailable | Coordinator: run full tools pytest, review/land, then restart dispatcher under coordinator ownership. |

## Delivered

Original-family logical **2/3**, not a fresh allocation. Original Claude-side round published `5cc36c5d` /
`72dc50ec` and landed train101; its copied public report remains unchanged. At most one later round remains.
Substantive implementation commit: `4f9b5601`. Final tested source (code/tests/operational docs):
**`0df3bc89faef0dfe93c17264f3da9ef7f9ab1a72`**. Subsequent report-only commit does not alter that source.

- `tools/lane_supervision.py`: separate bounded ownership lock, atomic fsynced journal/marker publication,
  bounded Windows sharing recovery, strict versioned incident validation and OS process identity acquisition.
- `tools/lane_heartbeat.py`: every successful snapshot journals capacity at the selected root's ignored
  `build-runs/capacity-supervision/incident-v1.json`. Distinct registered actual worker PIDs count; helpers,
  pending/draft entries, unregistered helpers and queue entries do not. Configured concurrency is the target
  (default nine); no cap or launch guard changes. Older heartbeat snapshots never backdate incidents.
- Journal records first/last observation epochs, age, target/live/open slots, launchable/waiting counts, exact
  launchable deficit/shortfall, suppression and explicit resolution. Covering launchable backlog retains shortage.
  Only actual recovery resolves; healthy initial observation invents no incident. Resolved evidence is retained.
  Age is observation span, including paused time and unobserved gaps, not proof of uninterrupted shortage.
  Rollback retains first observation and last verified age, marks uncertainty and exposes null age/UNKNOWN.
- STOP/provider pause (5) precedes capacity (4), then train/file events (0). Eight workers with a covering backlog
  exits4 before polling in wait/periodic/once modes. Only healthy capacity may sleep. Train verdict bytes/mtime
  are never acknowledged or rewritten, so the next invocation can recover the event. Wait writes the capacity
  journal, not heartbeat report/log, canonical coordinator, queue, train or rounds.
- Invalid/old-schema/partial/unreadable journal evidence is retained with actionable exit1 and existing
  report/log unchanged. Writer-lock contention is bounded at0.5s; replacement retries at0.5s; artifact reads at
  five attempts separated by0.02s. Bounds exclude an already blocked OS call. Existing critical JSON bounded
  recovery, original diagnostics, caller deadline and STOP/provider recovery checks remain tested.
- `tools/lane_refill.py`: only after acquiring sole dispatcher lock, publishes actual PID, OS creation identity,
  OS argv/executable, canonical root/cwd, provider/role in `run4-refill-owner-v1.json` plus compatible bare PID.
  Queue-only/refused second invocations never publish; publication failure exits1 before launch. Normal exit,
  STOP, pause and crash retain markers; old cleanup cannot delete a newer owner. An unheld lock is unhealthy.
- Heartbeat `refiller` is VERIFIED only for matching OS identity, role/root/provider, legacy PID and held owner
  lock. Missing companion/creation, stale/reused PID, unrelated process, queue helper, wrong root/provider and
  access denial remain UNKNOWN with reason. Windows creation/executable/self argv use supported process APIs;
  remote argv uses bounded4s PowerShell CIM. Linux uses boot ID/start ticks and `/proc` argv/exe. No name fallback.
- Operational contract documented in `docs/lane-tools.md` and the relevant coordinating paragraph. No automatic
  planning/briefing/promotion, queue mutation or overriding dependency/CPU/disk/cap/role/STOP/provider guards.
  Backend/bootstrap/laneguard, lanes.py and canonical claims are untouched.

## Exact validation

Final checks ran **serially on committed source `0df3bc89`** using
`python build-runs/validate-capacity-supervision.py`. It printed per-command exact argv/source/exit/count/log path
and exited1 for missing pytest. Read-only Git commands later in that PowerShell call exited0; that wrapper
exit is not a full-suite PASS. Installed executable: `C:/Program Files/Python312/python.exe`.

| Command | Printed result | Exit |
|---|---|---|
| `python -m unittest discover -s tools/tests -p test_lane_supervision.py -v` | `Ran 24 tests in 7.382s`, `OK` | 0 |
| `python build-runs/run-focused-heartbeat.py` | `Ran 16 tests`, `OK`; existing functions via local stdlib fixture adapter, not pytest | 0 |
| `python -m unittest discover -s tools/tests -p test_lane_heartbeat_reads.py -v` | `Ran 10 tests`, `OK` | 0 |
| `python -m pytest -q tools/tests` (ONE full-suite attempt) | `C:\Program Files\Python312\python.exe: No module named pytest` | 1 |
| `git -C <WT> diff --check` | no diff errors; only pre-commit LF/CRLF advisory warnings | 0 |

Unabridged outputs and machine-readable exact argv/results/source SHA are local ignored artifacts:
`build-runs/capacity-validation/{supervision,heartbeat-functions-adapter,critical-reads,full-pytest-attempt}.log`
and `results.json`. Adapter/validation scripts remain in this worktree's `build-runs/`.
**50 focused checks passed (24+16+10); full tools suite has not run.** Neither main nor worktree
`.venv/Scripts/python.exe` exists; installed Python312 lacks pytest. Cargo/clippy/ctest, native build, game and
REA/cache queries: n/a, deliberately not run for this tooling change. No ABI changes or questions filed.

Measured regressions: restart at1040 retains first1000/age40; eight live plus launchable ninth still exits4
with actual deficit1/launchable deficit0; twelve dependency-waiting briefs do not cover a slot; helpers/drafts/
duplicate PIDs do not inflate live. Train+shortage exits4 without losing verdict, healthy nine recovers it.
STOP/pause exits5 with active incident retained and no spawn; rollback retains last verified age50 and exposes
UNKNOWN. Tests cover malformed/partial/old/null schemas, retained read/replace failures, atomic concurrent readers,
bounded contention and serialized overlapping observers, wrong root/role/provider, stale23912/reused PID,
missing identity, AccessDenied/unsupported identity, queue-only/second-owner refusal, publication failure/crash,
denied launch staying blocked and old-exit protection. An isolated subprocess executes actual refill main/lock/
publication flow and proves marker PID/creation/argv/executable match its real OS identity; a second subprocess
cannot overwrite it. Only test-owned temporary roots/processes/locks were used, cleanup is bounded.

## Historical diagnostics and limits

- Supplied coordinator evidence, not remeasured here: stale23912 pointed away from live28668 until coordinator
  verification/manual correction. A bare alive PID is insufficient. Bare tool wrapper can transform4 to1;
  `python tools/lane_heartbeat.py --once; exit $LASTEXITCODE` preserves4, not a reason to reinterpret all1.
  Critical JSON errors preserve original paths, attempts, exception types/messages and deadline information.
- Exploratory critical-read run initially had seven stale assertion failures (old PID-only health/read counts),
  now all10 pass. Initial new16 run had two failures (Windows sharing read, global sleep mock blocking bounded
  retry) and one CIM error. Fixed through bounded sharing recovery, distinguishing poll from short retries,
  and supported direct Windows self APIs. Original CIM diagnostic: `PermissionDenied`, `HRESULT 0x80041003`
  from `Get-CimInstance`. Remote argv can still be denied here and must be UNKNOWN: no live remote-health claim.
  Windows self identity and real owned subprocess publication pass; Linux is unrun in this Windows environment.
- `q.py` could not start Bash: `Access is denied.` / `Error code: Bash/Service/CreateInstance/E_ACCESSDENIED`,
  `q: exit=1`; checks therefore used direct PowerShell and their printed results, not wrapper status.
- Each implementation commit printed `error: Unable to create 'C:/Users/tlori/FreeCity4/.git/packed-refs.lock':
  Permission denied` but successfully created detached commits `4f9b5601` and `0df3bc89`. No shared-ref retries,
  branch switches, merges, rebases, maintenance or pushes. Worker did not operate live dispatcher, workers,
  production locks/state/queue/rounds, main, engine, manual gates or generated tables.

## Landing and next steps

**WIP solely pending required full tools pytest validation in the coordinator's actual environment.** Queue
state is wip, not READY. Run `python -m pytest -q tools/tests` with pytest available against this exact source,
inspect count/results and cross-tool regressions, then mark ready/review/land. After landing, only the coordinator
may restart the normal dispatcher so it publishes its identity companion. No self-restart or extra round taken.

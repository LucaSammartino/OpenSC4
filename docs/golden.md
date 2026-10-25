# Original-game golden cache

`tools/golden.py` is independent of `play.py` and `suite.py`. It uses only Python's
standard library and the existing PowerShell game queue. It never writes saves
in Documents. A queue export holds both slot locks and verifies that the slot's
log still matches the completed run; an export race fails instead of caching a
different run's save.

```powershell
python tools/golden.py determinism --region "FC4 Test" --city Konradshohe --until 2026-04-01 --repeats 3
python tools/golden.py record --region "FC4 Test" --city Konradshohe --until 2026-04-01
python tools/golden.py record --region "FC4 Test" --city Konradshohe --until 2026-04-01 --rerun
python tools/golden.py compare --region "FC4 Test" --city Konradshohe --until 2026-04-01 --dll C:/path/to/worktree --replace simulator,traffic
```

Use `--original-dll` to identify the coordinator's observer DLL when it is not
`<main>/engine/build-x86/FreeCity4.dll`. `--dll` accepts either a worktree containing
that build path or a DLL file. `--script` accepts the plugin's dated grammar;
Lua requires the luacmd interface and is refused explicitly for now. A terminal
pause, digest, full save and completion mark are appended. Both the log and the
saved simulator date must reach exactly `--until`. Stalled, incomplete, overshot
or wrong-subsystem runs cannot be published.

`--turbo` requests turbo but **the original simulator cannot implement it**;
the plugin logs that it has no effect when `disable = all`. The default uses
public speed 2 with `pacing = original` for both runs. `session_seed` fixes our
generators, not the game's clock-seeded generators. An all-original baseline
does not currently provide a deterministic regression oracle. Repeated runs
measure this instead of hiding differences behind tolerances or ignore masks.

The SHA-256 key covers the executable, installed DAT/DLL/SGR/INI/CFG files,
observer DLL, start save, entire source region (including neighbours), user
configuration, source script, generated terminal script and harness sources and
settings. Candidate DLL bytes and replacement keys are excluded intentionally.
Queue slot selection is recorded as a policy, with the actual slot in each run.
Source hashes are checked again before publishing a baseline. Original and
candidate runs explicitly disable known shadow/compare settings.
Live runs reject game/region roots that differ from the existing harness's
actual launch and mirror sources; these options cannot redirect just the hash.

`build-runs/golden/<key>/` contains immutable `end.sc4`, `digests.json`,
`verdict.json`, `summary.json`, `game.log`, `harness.txt` and `baseline.json`.
An existing entry is verified and reused without launching a game. `--rerun` or
`determinism` compares an independent original run without overwriting it.
Each audit is retained as `repeat-<id>.json`; any differing audit permanently
qualifies that baseline as nondeterministic. Raw attempts, including failures,
are in `build-runs/golden-runs/`. Save files remain untracked user content.

Comparisons launch only the candidate, validate cache artifact hashes, and write
`comparison.json` naming the first differing digest date/fields and every
changed full type/group/instance. Saves are compared after DBPF index matching
and QFS decompression. Only container layout/header and DIR compression metadata
are omitted; **no gameplay field, thumbnail or counter is ignored**. Unknown
records include exact sizes, hashes, changed-byte counts and the first offset.
Candidate saves that overshoot the window still produce diagnostic differences,
but are labelled `invalid-window` and return 2; they cannot pass or be cached.
Field diagnostics decode exemplars/cohorts and LTEXT as in `engine/formats`, and
the simulator's v4 state as in `engine/core/src/save.rs`. A difference prints a
`play.py --profile shadows` replay command from the same start save through the
first differing date, retaining earlier simulation history. The suite owner
must align these arguments with the eventual `play.py` CLI, which is absent on
this base.

Exit codes: 0 = recorded/cached/exact, 1 = candidate differs, 2 = invalid or
incomplete run/input, 3 = nondeterministic original baseline (also for a
candidate that happens to match it). An unverified single recording is labelled
`unverified`; one successful repeat is `reproduced`, never a proof of general
determinism. A nondeterministic baseline cannot produce a passing compare.

Suite integration APIs: `fingerprint`, `run_game`, `cache_record`,
`compare_artifacts`, `qualification`, `parse_digests`, `shadow_command`, and
`golden_dbpf.compare_saves`. `cache_record` expects a fully validated run.
`--backend direct --slot N` uses `golden_run.ps1` with the queue's claim mutex;
it also needs the existing harness's process-inspection privileges. Region
requests fail explicitly until the coordinator queue supports its `region` key.
Reach artifacts are retained if supplied to `cache_record`; automatic reach
export awaits the main queue's per-run reach artifact interface.

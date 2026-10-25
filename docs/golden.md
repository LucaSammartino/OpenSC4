# Original-game golden cache

`tools/golden.py` is independent of `play.py` and `suite.py`. It uses only Python's
standard library and the existing PowerShell game queue. It never writes saves
in Documents. The exporter observes the run's unique script in the active slot
log, queues behind its user-dir lock before completion, and verifies the completed
queue log before copying. A later slot runner waits for that copy. A missed
marker or reused slot fails instead of caching a different run's save. The
runner mirrors any selected source region into its slot's `FC4 Test` folder;
the exporter uses that actual destination.

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

The default `--clock original` leaves **every subsystem original**, with no
shadows or compares. `record` launches one original, then reuses that immutable
cache without another game. `determinism --repeats 3` performs three independent
original runs. Comparison is byte-exact by default; differing original runs
mark the key nondeterministic and prevent a passing regression verdict (exit 3).
Digest and record differences are still reported as diagnostics.

The explicit experiment `--clock fixed` uses our scheduler with `sim_fixed_step = 1`,
`pacing = original`, and every other subsystem original. The scheduler advances
at most 64 agent deliveries per frame and yields at every new day, independently
of wall time and the turbo frame budget. The DLL must acknowledge this switch
in its log. `--clock original` retains the all-original wall-time audit;
`--turbo` cannot speed up that original scheduler. Both choices are in the key.
`session_seed` fixes our generators, not the game's clock-seeded generators.
Foundations §6.2 lists independent member, file-local and temporary generators;
no shared public seed interface is specified. We do not patch their memory.
Original agents' own timers, rendering and RNGs can still vary.

Raw save variation also includes identity encoding: foundations §2.7 specifies
that original object IDs are heap addresses, and §§3.3–3.4 specify entry checksums
that cover those IDs. Neither is controlled by a simulation seed. Header-only
diagnostic projections may quantify this, but body references and object order
also need format-aware treatment. The comparator retains them all; it does not
guess which payload words are references. Inspect monthly digest drift separately
from raw varying-record and opaque-byte counts.

Finish builds before a repeatability study and pin `--original-dll` to an
immutable copy, such as the content-addressed DLL under `build-runs/golden-inputs/`.
Source changes during a run are rejected. Candidate DLLs remain separate inputs
and never widen the original envelope.

The SHA-256 key covers the executable, installed DAT/DLL/SGR/INI/CFG files,
observer DLL, start save, entire source region (including neighbours), user
configuration, source script, generated terminal script, queue/runner/exporter sources and
settings. Candidate DLL bytes and replacement keys are excluded intentionally.
Queue slot selection is recorded as a policy, with the actual slot in each run.
Source hashes are checked again before publishing a baseline. Original and
candidate runs explicitly disable known shadow/compare settings.
Candidate DLLs are pinned atomically before launch; `summary.json` names the
staged path and SHA-256 of the bytes used, even if a later build changes the source.
Live runs reject game/region roots that differ from the existing harness's
actual launch and mirror sources; these options cannot redirect just the hash.

`build-runs/golden/<key>/` contains immutable `end.sc4`, `digests.json`,
`verdict.json`, `summary.json`, `game.log`, `harness.txt` and `baseline.json`.
An existing entry is verified and reused without launching a game. `--rerun` or
`determinism` compares an independent original run without overwriting it.
Each audit is retained as `repeat-<id>.json`; independent originals and their
hashes are retained under `samples/`. `determinism` remeasures an existing key.
For exploratory drift analysis, `--comparison envelope` explicitly requests
observed ranges. With that option, `determinism` adds runs until the set of
varying fields stops growing (up to `--max-repeats`, default 9). The envelope is
not evidence of deterministic simulation: decoded numeric fields and monthly digest components use
min/max; other decoded values use observed sets; unknown save bytes use explicit
per-offset min/max with record presence and size checks. Stable fields remain
exact; there are no gameplay masks. Numeric decoder spans are checked as fields
to avoid byte-carry artefacts. `envelope.json` reports varying records, decoded
fields, opaque byte fields, digest spreads and median decoded numeric spread.
Ranges are marginal observations; correlations and future unseen variance are
not inferred. Three runs or a single no-growth step cannot prove determinism.
Sampling that still grows at the limit is provisional (exit 3), never a pass.
Raw attempts, including failures,
are in `build-runs/golden-runs/`. Save files remain untracked user content.

Comparisons launch only the candidate, validate all original artifact hashes,
and write `comparison.json` naming the first differing digest date/fields and every
changed full type/group/instance. Saves are compared after DBPF index matching
and QFS decompression. Only container layout/header and DIR compression metadata
are omitted; **no gameplay field, thumbnail or counter is ignored**. Unknown
records include exact sizes, hashes, changed-byte counts and the first offset.
Candidate saves that overshoot the window still produce diagnostic differences,
but are labelled `invalid-window` and return 2; they cannot pass or be cached.
Field diagnostics decode exemplars/cohorts and LTEXT as in `engine/formats`, and
the simulator's v4 state as in `engine/core/src/save.rs`. A difference prints a
`play.py --profile shadows` replay command from the same start save through the
first differing date, retaining earlier simulation history. Its flags match the
current `play.py` CLI; suite/play sources remain owned by the suite lane.
Explicit envelope comparisons rebuild ranges only from originals, never from
the candidate, and label their verdict `within-envelope` rather than `exact`.

Exit codes: 0 = recorded/reused cache or compatible candidate,
1 = candidate differs, 2 = invalid or incomplete run/input, 3 = insufficient
or known original nondeterminism (also insufficient/still-growing sampling in
explicit envelope mode). `baseline_repeatability` retains the raw finding;
unverified means no independent repeat has established reproducibility.

Suite integration APIs: `fingerprint`, `run_game`, `cache_record`,
`compare_cached` (exact by default; `rule='envelope'` explicitly opts into ranges), `compare_artifacts`
(raw exact diagnostic), `qualification`, `parse_digests`, `shadow_command`, and
`golden_dbpf.compare_saves`. `cache_record` expects a fully validated run.
`--backend direct --slot N` uses `golden_run.ps1` with the queue's claim mutex;
it also needs the existing harness's process-inspection privileges. Region
requests use the coordinator queue's `region` key and mirror only slot copies.
Reach artifacts are retained if supplied to `cache_record`; automatic reach
export awaits the main queue's per-run reach artifact interface.

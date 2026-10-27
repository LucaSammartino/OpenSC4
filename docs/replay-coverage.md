# Replay-backed suite coverage

Record game inputs through the shared game queue, then replay without a game:

```powershell
python tools/suite.py record --set smoke --tag smoke-recording --reach --dll <recorder.dll>
python tools/suite.py replay smoke-recording
python tools/coverage.py --suite replay:build-runs/traces/smoke-recording/manifest.tsv
python tools/coverage_treemap.py --suite replay:build-runs/traces/smoke-recording/manifest.tsv
```

`record` requires `--reach`. Its trace and reach INI settings apply only to the
queued recording requests. Plugin defaults are unchanged. `--scenario` narrows
the selected set; `--dry-run` stages requests without submitting games or caching
a recording. The runner copies cities into isolated game slots; source cities in
Documents are read only. Recordings run one at a time through the shared queue.

Completed scenarios live under `build-runs/traces/store/<input-sha256>/`. Identity
includes the game executable, source city, scenario, script and Lua companion,
queue/runner scripts, reach contracts, profile and explicit settings. Repeating
`record` reuses completed scenarios for those inputs. Changing the replay core
DLL does not invalidate a recording; the Rust replayer caches verdicts using its
compiled source fingerprint instead. The recording DLL hash remains in provenance.
If a recording was interrupted, use `suite.py resume <record-tag>`, then repeat
`record` to finalize it. Failed/incomplete games are never cached as completed.

A set's `manifest.tsv` references SHA-256-named trace blobs, hard-linked from the
store when possible, otherwise copied. It has six columns without a header:
`scenario key trace_sha comparisons mismatches incomplete` (tab separated).
`recordings.json` beside it ties each scenario to its original recording census:

```json
{"version":1,"recordings":{"berlin-spandau":{"suite":"record-<input-prefix>","identity":"<input-sha256>"}}}
```

`suite` may also be an absolute `build-runs/suite/<tag>` directory. This supports
legacy recordings held by another worktree without copying their raw game logs.
An optional top-level `"census":"<tag or absolute suite directory>"` pins fallback
evidence. Otherwise each untraced key uses its newest census containing actual
observations, searching this worktree and the main checkout by per-key file time.

`replay` builds the current core by default, uses all available CPUs, and writes
`replay-verdicts.tsv`, `replay-suite.tsv` and a dated execution log beside the
manifest. `--jobs`, `--no-cache` and an explicit `--replayer <fc4-replay.exe>` are
available. Exit 1 retains inexact/incomplete evidence; it is not a successful
strict run. Failed invocations cannot leave older verdict files looking current.

Coverage cross-checks both verdict files against every manifest trace and source
fingerprint. A traced key earns strict credit only when every scenario/trace is
exact, with positive comparisons and zero mismatches/incomplete steps, and its
recording games completed with reach and action evidence. Replay never creates
entry counters. An untraced key uses census evidence; an inexact traced key cannot
fall back to census exactness. Reach stays separate for each verdict source so
census counters cannot credit a replayed owner. The Markdown report lists the
source and recording readiness of every key. Legacy traces lacking recording
entry counters earn zero replay strict bytes even if their outputs replay exactly.

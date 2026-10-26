# Lane tools

`python tools/lanes.py <command>` (standard-library Python 3). Coordinator/tooling/RE reference;
engine lanes use only the prompt's `.claude/lane-bin/` wrappers.

Run it from any checkout. Anything that looks at the other lanes (`slots`, `check`, `monitor`, `ledger`) finds the
main checkout through git and reads its `build-runs/` and `.claude/worktrees/`. `--repo <main checkout>` points it
at another checkout, read-only.

| Command | Who | What it is for |
|---|---|---|
| `slots` | coordinator, before every spawn | `active N / cap M - spawn ok`; exit 0 only when a spawn is ok |
| `check <lane> [--deps a,b] [--touches k1,k2]` | coordinator, before every spawn | refuses a lane that would be wasted work; prints `ok` or one reason per line |
| `wait --verdict <label>` or `wait --file <path> [--pattern <re>]` | tooling and RE lanes, the coordinator (engine lanes use `.claude/lane-bin/qwait.py`: they do not run `tools/`) | one tool call instead of a polling loop |
| `monitor [--interval 30]` | coordinator, under the Monitor tool | one line per event, silence otherwise |
| `ledger [--out ...] [--md docs/history/token-ledger.md]` | coordinator, now and then | tokens per lane, merge share, tokens per coverage point |
| `brief <subsystem-or-key> [--kind engine\|re] [--focus <regex>]` | coordinator | at most 60 lines: spec sections/ranges/sizes, question/core/ABI/conformance pointers, keys and coverage; `--focus` marks task sections |

## The cap rule

```
cap = min(20, logical CPUs, game-queue slots * 2 + 8)
```

Game slots come from `--game-slots`, a running queue's `-Slots` / `-SlotList`, the script default, or 6.

An **active lane** is a worktree under `.claude/worktrees` whose branch is not an ancestor of `main` and in which
some file changed in the last 30 minutes (build caches are skipped; the worktree's git HEAD, index and reflog count as
changes). A worktree that has not committed yet counts while it is new or has uncommitted changes.

`slots` prioritizes `low disk` (under 40 GB free), `at cap`, `cpu busy` (85% load, best effort), then `spawn ok`.
`-v` shows inputs/lanes. Spawn on exit 0 only, one at a time: an unchanged new lane is not yet counted.

## Waiting: one call, never a polling loop

Engine lanes use the [prompt's wait command](../.claude/workflows/fc4-lane.js). Tooling/RE/coordinator alternative:

```
python tools/lanes.py wait --verdict <label> --timeout 540
```

Internally checks every 5 s; prints at most 10 key verdict/problem lines. Exit 0 means the verdict exists,
not that it passed. Exit 2 means `timeout`; repeat the same wait. Keep `--timeout` at most 570 s.

`wait --file <path> --pattern <regex>` does the same for any output file (a gate log, a regress `.out`): it waits for
the file to exist and for a line to match, then prints the matching lines (without `--pattern`, the last lines).
PowerShell `*>` output is UTF-16; the tool detects the BOM, or NUL bytes, and decodes it. Use `--fresh` when an old file
of the same name may be there. The file is resolved against the checkout that holds the tool (your worktree); the
coordinator can add `--lane <name>` to look in another worktree.

## Coordinator

- Launch procedure: [coordinating.md](ops/coordinating.md#spawning-a-lane). `check` detects overlapping
  `--touches` keys in unmerged lanes active within 3 days or with ready/wip entries. Keys are case-insensitive
  substrings; review hits for duplicate work.
- `monitor` emits `queue:`, `game:` (nonclean or nonzero problems), `train:` (final verdict), `main:` and `stalled:`.
  Stalls mean no queue/file changes for 60 minutes, reported once within 8 hours; startup state is the baseline.
- `brief` reads `tools/coverage-map.tsv`; engine output filters RE paths, notes and executable addresses.
- **Cost:** `ledger` reads the workflow journals (`%USERPROFILE%\.claude\projects\<slug>\*\workflows\wf_*.json`) and
  writes `build-runs/lane-ledger.tsv` (one row per run), `build-runs/lane-ledger-by-lane.tsv` and, with `--md`,
  [history/token-ledger.md](history/token-ledger.md). Caveats: train landings lack merge journals;
  coverage-tool changes also move coverage.

## Heartbeat backlog and `--wait-until`

`python tools/lane_heartbeat.py --once` writes `build-runs/run4-heartbeat.json` and one log line (scheduled every
5 minutes). Briefs in `build-runs/run4-refill-queue.json` are launchable only when every `args.deps` is an ancestor
of `main` (`lane_refill.unmet_deps`); otherwise waiting. Output is `pending L launchable / W waiting [brief: deps]`.
JSON keys: `launchable`, `waiting` (brief to unmet deps), `open_slots` (target minus live).
`BACKLOG LOW: L launchable < N open slots` fires except during STOP/provider pause. Coordinator waits:

```
python tools/lane_heartbeat.py --wait-until train [--fresh] [--timeout 540]
python tools/lane_heartbeat.py --wait-until backlog
python tools/lane_heartbeat.py --wait-until build-runs/gate-train<N>.out
```

It checks every `--poll` seconds (default 10), prints nothing meanwhile, and on the first of these prints one line
and exits:

| Exit | Line | When |
|---|---|---|
| 0 | `TRAIN <id>: gate FAIL`, `TRAIN <id>: gate ok, regress <verdict>`, `TRAIN <id>: no longer current (landed or cleared)` | `train`: the current train in `build-runs/merge-train/state.json` needs the coordinator (`RUNNING` does not count) |
| 0 | `FILE <path>: present` | a file path (relative to `--root`: the checkout holding the script, the main checkout for the coordinator) exists |
| 4 | `BACKLOG LOW: ...; live x/y, pending L launchable / W waiting [...]` | the heartbeat alert above |
| 5 | `HALT: STOP set` / `HALT: provider <p> paused` | the STOP flag or a provider pause |
| 2 | `TIMEOUT after <s>s waiting for <event>; ...` | nothing happened; call it again |
| 3 | `WAIT: ...` | no current train for `train`, or `--timeout` outside (0, 3600] |

Existing verdict/file/low backlog returns immediately. `--fresh` ignores prior verdicts/files for reruns;
`backlog` waits only for backlog. Default timeout 540 s, maximum 3600. Waits do not write heartbeat files.

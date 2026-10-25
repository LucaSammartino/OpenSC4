# Lane tools

`python tools/lanes.py <command>` (standard-library Python 3). It exists to cut wasted agent tokens: lanes were
spawned into a full machine (about 25 lanes slowed the regress from 4.81 to 2.19 game days/s), agents burned turns
polling, the coordinator re-read status it could have been told about, lanes started before their dependencies
landed, and nobody knew what a lane cost.

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
| `brief <subsystem-or-key> [--kind engine\|re] [--focus <regex>]` | coordinator, when writing a lane prompt | a context pack of at most 60 lines to paste into the prompt, with each spec's section outline (line ranges and sizes); `--focus` marks the sections and subsections the task is about, so the lane reads those ranges instead of a 100-200 KB spec |

## The cap rule

```
cap = min(16, logical CPUs * 3 // 4, game-queue slots * 2 + 4)
```

On the 20-thread machine with the default 3 game slots that is `min(16, 15, 10) = 10`; with 6 slots it is 15. The game
slots come from `--game-slots`, else from the `-Slots` / `-SlotList` of a running `game_queue.ps1`, else from the
`-Slots` default in `tools/game_queue.ps1`, else 6.

An **active lane** is a worktree under `.claude/worktrees` whose branch is not an ancestor of `main` and in which
some file changed in the last 30 minutes (build caches are skipped; the worktree's git HEAD, index and reflog count as
changes). A worktree that has not committed yet counts while it is new or has uncommitted changes.

`slots` prints, in this order of priority: `low disk` (under 40 GB free on the repo drive; each engine worktree's
build caches are 10-19 GB), `at cap`, `cpu busy` (CPU load at 85% or more, best effort), else `spawn ok`. `-v` also
prints the inputs and the active lanes. Spawn only on exit 0, and spawn one lane at a time: a lane that has not
written a file yet is only counted from its first change.

## Waiting: one call, never a polling loop

A game run goes through the queue, which takes minutes. Do not poll with `sleep` turns or repeated `Read`s: every
poll is a turn that costs tokens and finds nothing. Engine lanes wait with `.claude/lane-bin/qwait.py`
([harness.md](harness.md) "How long to wait"). Tooling and RE lanes submit the request, then make **one** call:

```
python tools/lanes.py wait --verdict <label> --timeout 540
```

It blocks inside the process (it looks at the filesystem every 5 s and prints nothing meanwhile), and when
`build-runs/game-results/<label>.verdict` exists it prints at most 10 key lines (the verdict word, the autotest line,
the request, the city and DLL, any non-zero problem counter, the simulator and game lines) and exits 0. A verdict word
other than `clean` is still exit 0: read the line. On **exit 2** it printed `timeout` and the run has not finished;
call the same command again. The limit is under the 10-minute tool-call limit, so do not raise `--timeout` above 570.

`wait --file <path> --pattern <regex>` does the same for any output file (a gate log, a regress `.out`): it waits for
the file to exist and for a line to match, then prints the matching lines (without `--pattern`, the last lines).
PowerShell `*>` output is UTF-16; the tool detects the BOM, or NUL bytes, and decodes it. Use `--fresh` when an old file
of the same name may be there. The file is resolved against the checkout that holds the tool (your worktree); the
coordinator can add `--lane <name>` to look in another worktree.

## Coordinator

- **Before each spawn:** `slots`, then `check <lane> --deps <branches it builds on> --touches <keys of the code it
  changes>`. `check` fails (exit 1) when a dependency is not an ancestor of `main` yet, when the branch, the
  worktree or `docs/lanes/<lane>.md` exists already, or when another unmerged lane (one active in the last 3 days, or
  with a ready/wip queue file) mentions a `--touches` key in its `docs/lanes/*.md` or merge-queue file. A key is a
  case-insensitive substring: `power`, `cSC4Power`, a spec name. A hit means possible duplicate work, not certain.
- **While lanes run:** `monitor` under the Claude Code Monitor tool. It prints nothing until something happens, then
  one line each: `queue:` (a merge-queue file appears or changes state, or a lane is marked merged), `game:` (a verdict
  that is not a clean pass, or a clean one with a non-zero problem counter), `train:` (a `gate-train*` or
  `regress-train*` output got its final verdict line), `main:` (main moved: a batch landed), `stalled:` (an unmerged
  lane with no queue file has had no file change for 60 minutes; once per lane, only within the last 8 hours). The
  state at start-up is the baseline and is not reported.
- **Lane prompts:** `brief <key> --kind engine|re` and paste it in. The brief is built from `tools/coverage-map.tsv`:
  spec files with sizes and their `docs/specs/questions/<stem>.md` (and whether it exists), the engine core paths, the
  shadow and replace keys, how many `docs/abi` rows mention the owner classes (count, files and a grep command, not
  the rows), the conformance vectors and their runner, and the subsystem's current coverage status. With `--kind
  engine` it prints only what an implementer may read under the clean-room rules in CONTRIBUTING.md: no `re/`,
  `tools/`, decompiler or Ghidra paths, no analysis-side notes, no executable addresses (a final filter drops any line
  that would show one).
- **Cost:** `ledger` reads the workflow journals (`%USERPROFILE%\.claude\projects\<slug>\*\workflows\wf_*.json`) and
  writes `build-runs/lane-ledger.tsv` (one row per run), `build-runs/lane-ledger-by-lane.tsv` and, with `--md`,
  [token-ledger.md](history/token-ledger.md). Read the caveats there before quoting tokens per coverage point: lanes landed
  through the merge train have no journal, and coverage also moves when the coverage tool changes.

## Run 4: the heartbeat backlog and `--wait-until`

`python tools/lane_heartbeat.py --once` (the five-minute scheduled task) writes `build-runs/run4-heartbeat.json` and
one log line. Pending briefs in `build-runs/run4-refill-queue.json` are split the way the refiller decides:
a brief is **launchable** when every one of its `args.deps` is an ancestor of `main` (`lane_refill.unmet_deps`, the
refiller's own `git merge-base --is-ancestor` rule), otherwise it is **waiting**. The line reads
`pending L launchable / W waiting [brief: dep,dep; ...]`, and the JSON has `launchable`, `waiting` (brief to the deps
it waits on) and `open_slots` (target minus live). On top of the existing alerts, `BACKLOG LOW: L launchable < N open
slots` fires whenever the refiller has fewer briefs it can start than free slots (not while STOP or a provider
pause is set). It was added after run 4 drained from 9 live lanes to 1 with `pending 2` in the log: both briefs
depended on the train that was waiting to land, so neither could start, and the old `REVIEW QUEUE` alert only fires
at 0 pending.

The coordinator's long waits go through it:

```
python tools/lane_heartbeat.py --wait-until train [--fresh] [--timeout 540]
python tools/lane_heartbeat.py --wait-until backlog
python tools/lane_heartbeat.py --wait-until build-runs/gate-train100.out
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

A verdict or file already there returns at once (exit 0), and so does a backlog that is already low (exit 4): that is
the point, the wait never starts while slots are going to empty. `--fresh` ignores a train verdict or file that was
there before the call (for a re-run gate, whose old `gate.ok: false` stays in the state file until the new one is
written). `backlog` waits for the backlog only. The default `--timeout` is 540 s, under a 10-minute tool-call
limit; the maximum is 3600. The wait is read-only: it does not write the heartbeat files.

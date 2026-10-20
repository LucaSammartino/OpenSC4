# Coordinating lanes

Rewritten from HANDOFF §6 on 2026-10-20 for the merge train; the old lock and merge-order procedure is kept, as it
was, in [docs/history/handoff-2026-10.md](../history/handoff-2026-10.md). Who reads it: the coordinator (the one
owner of `main`). Lanes read [docs/lanes/README.md](../lanes/README.md) and [docs/merge-train.md](../merge-train.md)
instead: write the lane file, enqueue, stop.

## Spawning a lane

Every lane is one run of the saved workflow [`.claude/workflows/fc4-lane.js`](../../.claude/workflows/fc4-lane.js):
`Workflow({name: 'fc4-lane', args: {kind, name, task, date, brief, deps, game, fix?, model?, effort?}})`. It has no
merge step: the lane writes `docs/lanes/<lane>.md` (row + full report) and `build-runs/merge-queue/<lane>.json`,
returns a short result (state, tip, one-line gate/game/questions, at most 5 next steps) and stops. Before each spawn
([docs/lane-tools.md](../lane-tools.md)):

1. `python tools/lanes.py slots --game-slots 6` must say `spawn ok`. The cap exists because ~25 lanes on 20 threads
   halved the regress speed (4.81 to 2.19 game days/s) and every agent then spent turns waiting.
2. `python tools/lanes.py check <lane> --deps a,b --touches key1,key2` must say `ok`: dependencies landed on main
   (spawn in order core, then its DIFFERS/answers, then glue), no existing branch or lane file, no other active lane
   on the same keys (two riot-lock implementations came from skipping this). Do not spawn speculatively near a
   planned stop: a lane stopped midway is mostly wasted.
3. `python tools/lanes.py brief <subsystem> --kind engine|re`, pasted as `args.brief`: the spec, its questions file,
   core paths, keys, ABI and conformance pointers. Lanes read the brief and the one-page HANDOFF, not whole trees.
4. Pick the model: `kind: 'mech'` (docs rows, ABI rows, regress records, regenerations) runs on Sonnet by default;
   RE, spec and engine work keep the session model. Pass `effort: 'low'` for mechanical steps.

The template's long rules block comes first and is byte-identical for every lane of a kind (no name, date or path
in it), so it is served from the prompt cache; keep it that way when editing. A fix round is the same call with
`fix` set to the train's failure text. Engine lanes test their own DLL through the run queue
(`tools/game_queue.ps1 -Slots 6`, started detached from the main checkout; [docs/harness.md](../harness.md)) and wait
for verdicts in one blocking call, never a polling loop. A worktree lane cannot read files outside its worktree:
paste what it needs into the task, or copy files into its gitignored `build-runs/`.

## Keeping the coordinator cheap

- **One monitor, events only:** run `python tools/lanes.py monitor` under the Monitor tool for the whole session. It
  prints a line only when a lane enqueues, a verdict is not clean, a train gate/regress finishes, `main` moves or a
  lane stalls; do not re-arm timers or poll status between events.
- **Read results, not transcripts:** the lane's structured result and `docs/lanes/<lane>.md` are the record; never
  read an agent's output file. Batch status updates to the user instead of one per event.
- **Measure:** `python tools/lanes.py ledger --md docs/token-ledger.md` after each landed batch; it reads the
  workflow journals and puts tokens per lane, merge share and tokens per coverage point in
  [docs/token-ledger.md](../token-ledger.md). Quote it, not estimates.

## Landing work: the merge train

The contract is [docs/merge-train.md](../merge-train.md); the tool is `tools/merge-train.py`, run from the main
checkout on `main`. A finished lane writes `build-runs/merge-queue/<lane>.json` in its own worktree and stops.

```
python tools/merge-train.py status                  # queued lanes, and entries that are not really ready
python tools/merge-train.py plan --size 12          # the next batch
python tools/merge-train.py merge-batch --size 12   # merge onto integration/<n> in .claude/worktrees/train-integration
python tools/merge-train.py gate                    # once per batch
python tools/merge-train.py regress                 # once per batch (engine lanes)
python tools/merge-train.py land                    # fast-forward main; notices in build-runs/merge-notices/
python tools/merge-train.py bisect | fail --lane L  # a red batch
```

- **Verify every landing:** `git merge-base --is-ancestor <lane> main` for each lane the train lists as landed.
  Never trust the train's own lane list (eight `train3` lanes were reported landed and were not; see
  [HANDOFF](../HANDOFF.md) "Current state").
- **Gate.** The train's `gate` runs `cargo test` and `cargo clippy -D warnings` from `engine/`, then
  `build-runs/coord-verify-lane.ps1` (CMake build + ctest, a private `_MSPDBSRV_ENDPOINT_`), and judges the parsed
  `build exit=`, `ctest exit=` and ctest summary, never the wrapper's exit code. By hand, for one worktree:
  `build-runs/coord-full-verify.ps1 -Engine <worktree>\engine -Tag <tag>`. Then `rustfmt --check --edition 2021` on
  the touched `.rs` files from `engine/`. The history/clock24 thread tests were fixed by `flakyfix` (25/25 each
  under an 18-process CPU load, 2026-10-20); a failure there is now a real one.
- **Conflicts the train refuses** go to the operator (see the notes below). Resolve in the integration worktree with
  `git -C <wt> -c submodule.recurse=false ...` and add files explicitly (`commit -a` fails on the worktree's
  submodule pointer). Renumber seam ops / switch counts there if two lanes took the same one. The ABI version is
  bumped once per batch by `python tools/bump_abi.py --batch <batch-id> --lanes laneA,laneB` (the train calls it;
  never per lane, never by hand). `abi_entries.inc` is generated at build time and not in git: never merge,
  renumber or revert it. Write merge messages to a file and use `-F <msgfile>`.
- **After a landing:** the regress ran in the batch (read the shadow column, not only the verdict); run records go
  in `docs/runs/<tag>.md` and `python tools/gen_handoff_tables.py` regenerates `docs/lanes/INDEX.md`, the HANDOFF's
  latest-run line and `docs/manual-checks.md`; questions: see docs/specs/questions/README.md; tick
  [docs/PROGRESS.md](../PROGRESS.md); regenerate `docs/coverage-report.md` and `docs/coverage-map.svg`
  (`python tools/coverage.py`, `python tools/coverage_treemap.py`) and check the delta: it must never go down.
- **Pause / stop:** `TaskStop`, resume with `SendMessage` to the lane's agent ID (but never `SendMessage` a
  workflow subagent: that spawns a duplicate). After a hard stop, check the worktree's *unstaged* diff
  (`git -C .claude/worktrees/<lane> status`): the agent may have been writing the fix to something it had just
  committed.

## Merge-train operator notes

Learned the hard way on 2026-10-16:

- `build-runs/coord-verify-lane.ps1` **exits 0 even when cmake fails**. Never take a gate from the wrapper's exit
  code — `merge-train.py`'s `gate` parses `build exit=`, `ctest exit=` and the ctest summary.
- Lanes append to shared files (`fc4.h`'s ABI history, `FC4Config.*`'s key table, `FC4Subsystems.cpp`'s registry),
  and **each side's addition ends with the file's own closing `}`/`*/`**. The resolver now splits that close off the
  last line, commas list entries only, checks the three-way brace balance, and rejects glued `fn` signatures. It also
  refuses to change anything outside `fc4.h`'s history block and collapses a duplicated `FC4_ABI_VERSION` — a bug
  there once deleted all 126 `FC4_ADV_SRC_*` defines and 308 declarations.
- `git checkout --theirs -- fc4.h` during a manual resolve is dangerous: the lane's copy is **stale** (cut from an
  older main) and drops everything merged since. Restore from the consolidate commit instead.
- `reset_integration` (`checkout --force -B integration/<n> main`) discards a previous attempt's merges, so a batch
  re-run from scratch can record a lane as merged whose merge did not survive; `land` writes its list from its own
  state file, which is not a proof of ancestry.
- `docs/HANDOFF.md` and `docs/lanes/INDEX.md` conflicts are taken from the integration side and regenerated (a lane
  cut before 2026-10-20 carries the old 134 KB HANDOFF; a union would paste it back).
- PowerShell `*>` output in `build-runs/` is UTF-16; `tr -d '\0'` before grepping.
- A git worktree's `engine/third_party/gzcom-dll/.git` holds a relative `gitdir:` that does not resolve — repoint it
  at `<main>/.git/modules/engine/third_party/gzcom-dll` or every git command in that worktree fails.
- Disk: build caches are ~10-19 GB per worktree. `C:` hit ENOSPC twice mid-gate. Free
  `engine/target` and `engine/build-x86` of **merged, clean, 0-commits-ahead** worktrees only.

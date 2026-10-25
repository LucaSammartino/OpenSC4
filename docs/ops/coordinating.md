# Coordinating lanes

Rewritten from HANDOFF §6 on 2026-10-20 for the merge train; the old lock and merge-order procedure is kept, as it
was, in [docs/history/handoff-2026-10.md](../history/handoff-2026-10.md). Who reads it: the coordinator (the one
owner of `main`). Lanes read [docs/lanes/README.md](../lanes/README.md) and [docs/merge-train.md](../merge-train.md)
instead: write the lane file, enqueue, stop.

## Spawning a lane

**Provider selection:** the tracked `tools/lane_launch.py` and `tools/lane_refill.py` support explicit
`--provider claude|codex` with one shared ledger and all checks below. Claude uses the saved Workflow;
Codex uses its tested subscription CLI adapter and the same guard policy through registered hooks.
See [lane-providers.md](lane-providers.md) for commands, authentication, model settings and recovery.
Provider changes never reset rounds or parking, and never provide a fallback around a denial.

Every lane is one run of the saved workflow [`.claude/workflows/fc4-lane.js`](../../.claude/workflows/fc4-lane.js):
`Workflow({name: 'fc4-lane', args: {kind, name, task, date, brief, deps, game, fix?, model?, effort?, ctxBudget?,
maxCalls?, maxRounds?}})` (see "Lane context" and "Lane guard" below). It has no
merge step: the lane writes `docs/lanes/<lane>.md` (row + full report) and `build-runs/merge-queue/<lane>.json`,
returns a short result (state, tip, one-line gate/game/questions, at most 5 next steps) and stops. Before each spawn
([docs/lane-tools.md](../lane-tools.md)):

0. No STOP flag: `slots` and `check` both fail while `build-runs/STOP` exists, and the workflow refuses to start a
   round anyway ("Lane guard" below). A wind-down order is `python tools/lanes.py stop "<reason>"`, not a line in
   the HANDOFF.
1. `python tools/lanes.py slots --game-slots 6` must say `spawn ok`. The cap exists because ~25 lanes on 20 threads
   halved the regress speed (4.81 to 2.19 game days/s) and every agent then spent turns waiting.
   Raised 2026-10-23 from 15 to 20 (`min(20, cpus, game_slots*2+8)`) once native builds moved to tools/build_queue.py:
   with 15 unmerged lanes the regress held 3.3-3.9 days/s (shadows) and 16.8-20.2 (replace). If the shadows profile
   drops under 3.0 days/s, lower it again.
2. `python tools/lanes.py check <lane> --deps a,b --touches key1,key2 [--kind re]` must say `ok`: no STOP flag, the
   lane not parked, the work not assigned to an outside contributor ([docs/assignments.tsv](../assignments.tsv)), for RE lanes (`--kind re`, or any lane named `re-*`) a reachable REA server, dependencies landed on main
   (spawn in order core, then its DIFFERS/answers, then glue), no existing branch or lane file, no other active lane
   on the same keys (two riot-lock implementations came from skipping this). Do not spawn speculatively near a
   planned stop: a lane stopped midway is mostly wasted.
3. `python tools/lanes.py brief <subsystem> --kind engine|re --focus <regex>`, pasted as `args.brief`: the spec with its
   section outline (`--focus` marks the sections the task names; specs run to 100-200 KB), its questions file, core
   paths, keys, ABI and conformance pointers. Lanes read the brief and the one-page HANDOFF, not whole trees.
4. Pick the model: `kind: 'mech'` (docs rows, ABI rows, regress records, regenerations) runs on Sonnet by default;
   RE, spec and engine work keep the session model. Pass `effort: 'low'` for mechanical steps.

The template's long rules block comes first and is byte-identical for every lane of a kind (no name, date or path
in it), so it is served from the prompt cache; keep it that way when editing. A fix round is the same call with
`fix` set to the train's failure text. Engine lanes test their own DLL through the run queue
(`tools/game_queue.ps1 -Slots 6`, started detached from the main checkout; [running-the-game.md](running-the-game.md) "The game queue") and wait
for verdicts in one blocking call, never a polling loop. A worktree lane cannot read files outside its worktree:
paste what it needs into the task, or copy files into its gitignored `build-runs/`.
Codex lanes cannot run the native build in their sandbox (MSVC configure hangs), so the coordinator also keeps
`tools/build_queue.py --jobs 1` running detached from the main checkout; `qbuild.py quick|verify` hands the native steps to it
while `build-runs/build-queue-worker.alive` is fresh, and it waits while a game run is in progress (`--status` shows the queue).

## Lane context (2026-10-20)

The audit of 2026-10-20 (OpenRouter, `z-ai/glm-5.3-flash`, $184.71) found 85% of the spend in cached context
re-reads: lanes started big and grew without compaction. First-call context, measured from the `usage` of the first
assistant event of every subagent transcript under `%USERPROFILE%\.claude\projects\*\*\subagents\`:

| Route, agent | n | first call | peak per lane |
|---|---|---|---|
| GLM (`claude-desktop-3p`), `workflow-subagent` (today's lanes) | 23 | 172.3k-173.8k | 212k-975k |
| MiMo (`claude-desktop-3p`), `general-purpose` / `claude` / `Explore` | 80 | 172.7k-177.4k | |
| Claude Opus 5.5, `general-purpose` | 173 | 45.7k-55.0k | |
| Claude Opus 5.5, `workflow-subagent` | 220 | 49.7k-90.9k | |
| **After**, `fc4-lane-re` agent type (one probe) | 1 | see "Probe" below | |

Why: on the third-party route Claude Code has no tool search, so every MCP schema of the session (REA's ~170 tools,
Chrome, computer-use, the browser pane, the app's own servers) goes inline on every call; the Claude route defers
them behind ToolSearch, hence ~55k. Lanes also made 10,258 Bash calls (13 REA calls), one command per turn, each
re-sending the whole context. Three changes, on branch `claude/wonderful-lamarr-87a662`:

1. **Agent types.** [`.claude/agents/fc4-lane.md`](../../.claude/agents/fc4-lane.md) (`tools: Bash, Read, Edit,
   Write, Grep, Glob`) and [`fc4-lane-re.md`](../../.claude/agents/fc4-lane-re.md) (the same plus
   `mcp__rea__procedure_pseudo_code`, `mcp__rea__batch_decompile`); the workflow passes `agentType` (RE lanes get
   the second). A listed `tools:` is the agent's whole tool set, so no other schema is sent on either route.
   **Agent types are read once, when a session starts, from its cwd's `.claude/agents/`:** after this lands, start
   a new coordinator session in the main checkout. In an older session the workflow now stops with "agent type ...
   unknown to this session": a default subagent would run without the lane guard ("Lane guard" below). Pass
   `args.unguarded: true` to run it anyway as a default subagent (every schema again, no guard).
2. **Stateless lanes.** A lane keeps its working state in `docs/lanes/<lane>.md` (the row, State "in progress",
   then `## Handoff`: goal, done with hashes, next, facts; at most 25 lines; the generator ignores the section),
   commits it after every milestone, then runs `python <MAIN>/.claude/lane-bin/ctx.py <lane> --budget <n>`. Past
   the budget it returns `state: "continue"`. The workflow loops (`args.ctxBudget`, default 150000;
   `args.maxRounds`, default 6): `continue` or a null result (an API error after retries) starts a fresh agent with
   the same prompt plus a RESTART block that reads the Handoff, `git log main..` and `git status`; two nulls in a
   row stop (that is also what a user skip looks like). How the budget is detected: the workflow script has no
   file access and `agent()` returns no usage, so the lane measures itself: `ctx.py` finds the newest transcript
   whose `.meta.json` description is `lane:<lane>` or `lane:<lane>#<n>` and reads the last call's `usage` (0.1 s).
   Restart rounds are labelled `lane:<lane>#<n>`; `lanes.py ledger` already strips the suffix.
3. **Quiet output, several steps per call.** [`.claude/lane-bin/`](../../.claude/lane-bin/) (no RE content, so
   engine lanes may run it): `qbuild.py --wt <WT> test clippy verify | fmt <files>` (full logs in
   `<WT>/build-runs/qbuild-<step>.log`; prints counts, exit lines, the first 15 errors), `qgrep.py` (40 lines, 8
   per file, 160 chars, then a count of the rest; `-l`, `-C`), `qwait.py <file> [--fresh]` (one blocking wait, 9
   minutes). `tools/re_query.py asm` prints 56 lines per call (was 400, then 120) with `--skip N` paging; every re_query call is capped at 60 lines / 4 KB. The lane
   prompt's new "Cost rules" say: chain steps in one Bash call or a build-runs script, cap every output, Grep with
   `head_limit` <= 40, forward-slash paths (lanes have Bash, not the PowerShell tool).

The audit's replay of today's call sequences with a ~50k base and a 250k cap gave 38% of today's tokens; the 150k
default budget here is tighter.

**Probe** (one trivial agent; the first call is the whole fixed cost). From a session started after the agent files
exist in its cwd:
`Workflow({name: 'fc4-lane', args: {kind: 're', name: 'probe-ctx', maxRounds: 1, effort: 'low', task: "PROBE ONLY ...
return immediately: lane 'probe-ctx', tip 'none', state 'blocked', summary 'probe'."}})`, then read the first
assistant `usage` in `<session>\subagents\workflows\<run>\agent-*.jsonl` (`python .claude/lane-bin/ctx.py
probe-ctx --first` prints it). Run it once from a GLM-route session too: that route is the one that went from 172k.
Not yet verified there: that `tools:` also drops the MCP schemas on the third-party route (it should, since it
names the whole tool set and the route only changes deferral). If the GLM probe still starts near 172k, turn the
Chrome, computer-use and browser connectors off for the coordinator session and keep `rea` (top-level
`mcpServers` in `~/.claude.json`) only in sessions that spawn RE lanes.

## Lane guard (enforced, 2026-10-21)

The policy now has two transport adapters: Claude's original hook below and Codex's
`.claude/lane-bin/codexguard.py`. Both update the same per-lane counters and round/parking ledger.
Codex's first read-only identity probe initializes measured usage; missing later telemetry blocks work.
Its adapter and real watchdog denial were validated before enabling project lanes; see
[provider validation](lane-providers.md#validation-2026-10-22).

The context budget, the "stalled lane" rule and the wind-down order were prompt text, and the models ignored them.
Measured over the two GLM runs of 2026-10-20/21 ([efficiency-log.md](../history/efficiency-log.md)): 88% of lane context
tokens were spent above the 150k budget; re-answers35 made 2,789 calls with no output; re-specs22 lost $21 of work by
never committing; run 2's coordinator started three lanes after a wind-down order was compacted out of its context.
These rules are now enforced by the harness, the same on every model route.

**The hook.** [`.claude/settings.json`](../../.claude/settings.json) runs
[`.claude/lane-bin/laneguard.py`](../../.claude/lane-bin/laneguard.py) `hook` before every tool call of every session
in the checkout. It acts only on agents of type `fc4-lane` / `fc4-lane-re` whose workflow label is `lane:<lane>` or
`lane:<lane>#<n>`; everything else (the coordinator, the gate agent, other subagents) passes straight through on a
fast path. Why `settings.json` and not the agents' frontmatter `hooks:`: measured with Claude Code 2.1.304 on
2026-10-21, settings hooks fire inside subagents, Workflow agents included, and their input carries `agent_id` and
`agent_type`. Frontmatter hooks of a project agent are skipped until the folder is trusted (a `-p` session never
counts as trusted), which would leave the guard fail-open. The hook input's `transcript_path` is the *parent*
session's transcript. The guard finds the lane's own transcript and label beside it:
`<session>/subagents/workflows/<run>/agent-<agent_id>.jsonl` and its `.meta.json`.

| Rule | Trips when | The lane must | Persists |
|---|---|---|---|
| Context budget | the latest call's input (the lane's own transcript `usage`, as `ctx.py` reads it) is over `args.ctxBudget` (150000) | update the Handoff, commit it, return `continue` (the workflow restarts it fresh) | while over |
| Progress watchdog | more than `args.maxCalls` (150) tool calls since the worktree's HEAD last moved; the count is per lane, so it carries across restart rounds | commit its findings so far, return `blocked` with the reason and the question | for the rest of that agent's life, even after it commits |
| STOP flag | `<main>/build-runs/STOP` exists | commit the Handoff, return `blocked` (`STOP: ...`) | until `lanes.py go` |
| Parked | the lane's last two recorded results were both `blocked` on the same question IDs (or both watchdog trips); a result under STOP does not count | return `blocked` at once | until `lanes.py unpark <lane>` |

While a rule is tripped, the only calls allowed are these:
- Read, Write or Edit of `<WT>/docs/lanes/<lane>.md` and `<WT>/build-runs/msg-*.txt`.
- Bash made only of `git -C <WT> add|commit|status|log|rev-parse|diff` segments, optionally piped to `head` or `tail`.
  No `$(...)`, backticks, heredocs or redirects, and no other `-C` path.
- The final structured result.

The denial (`LANE GUARD - ...`) names the exact files, commands and the state to return. The limits reach the hook
through the lane prompt's `lane guard: budget=N calls=M` line, which the workflow writes from its args. The hook
records every lane's `StructuredOutput` (its return value) in `build-runs/lane-guard/<lane>.rounds.jsonl`, and
parking reads that ledger. State and logs, all under `build-runs/lane-guard/` (gitignored):
- `<lane>.json`: HEAD and the call count.
- `<lane>.log`: one line per denial.
- `agents/<id>.json`: the limits and any trip.
- `errors.log`.

A guard exception allows the call and is logged: a bug in the guard must not kill every lane. Hook timeouts (20 s)
also allow the call. `lanes.py slots -v` and `lanes.py guard` show recent errors.

**The workflow gate.** A workflow script has no file access, so before every round (start or restart) `fc4-lane.js`
runs a small gate agent (`gate:<lane>#<n>`, effort low, about one call). The gate runs
`laneguard.py preflight <lane> --kind <kind>` and reports its one line:
- `GO`: the round starts.
- `STOP ...`, `PARK ...` or `REA-DOWN ...` (RE lanes): the workflow returns `state: blocked`, `summary: "round <n> not
  started: ..."`, and starts no lane agent.

The hook is the backstop if the gate misreports. Even then, a lane agent under STOP or parked is denied from its
first call. `lanes.py ledger` counts gate tokens as the lane's.

**Commands.**

```
python tools/lanes.py stop "wind-down: land what is on disk"   # the STOP flag (JSON: reason, since, by)
python tools/lanes.py go                                        # clear it
python tools/lanes.py unpark <lane> [--note "..."]              # release a parked lane, zero its call count
python tools/lanes.py guard [<lane>]                            # STOP, calls since commit, last trip, parked
python tools/lanes.py check <lane> --kind re                    # adds the REA pre-flight
python .claude/lane-bin/laneguard.py preflight <lane> --kind re # what the gate agent runs
```

STOP blocks lanes only. `merge-train.py` still lands what is queued, and `status` prints the flag: a wind-down lands
what is on disk. Running lanes under STOP get the "commit the Handoff and return" denial on their next call. After
`go`, restart a lane with a normal spawn; its RESTART path reads the Handoff.

**REA pre-flight** (`check --kind re`, and the gate of every RE round). It fails in any of these cases:
- `~/.claude.json` has no `rea` server, or its node, script, Ghidra or Java path is missing.
- No `rea.mjs mcp` process is running, which means no Claude session holds an REA connection.
- A probe server started over stdio does not answer `initialize` + `tools/list` with `procedure_pseudo_code` and
  `batch_decompile` (about 2 s; no binary is opened and Ghidra is not started; `--no-rea-handshake` skips the probe).

It cannot see inside the coordinator's own Ghidra session. A session stuck on "Ghidra client is already started"
([docs/re-workspace.md](../re-workspace.md)) still passes. The watchdog is what stops an RE lane that then explores
without output.

**Tests.** `python -m unittest discover -s tools/tests` (`test_laneguard.py`). The tests use fixtures: fake subagent
transcripts in the 2.1.304 layout, a real git repo as the worktree, the hook run as a subprocess on the captured
input shape. They cover these cases:
- An over-budget transcript is denied, and committing the lane file still passes.
- N calls without a commit are denied, and the trip survives a commit for that agent.
- STOP denies lanes and fails `slots` and `check`.
- Two rounds blocked on the same question park the lane.
- The Bash allow-list holds.
- A dry run of `fc4-lane.js` under node (`fc4_lane_dryrun.mjs`, mock `agent()`, the gate's command run for real):
  it refuses to start with STOP, refuses to restart when STOP appears mid-run, restarts on `continue`, and refuses
  a parked lane.

End to end, once, in a scratch project (not this repo, no lane): a Haiku Workflow agent of type `fc4-lane` labelled
`lane:probe-guard` with `calls=2` ran two Bash calls. Its third was denied with the NO PROGRESS text, and its return
was recorded in the rounds ledger.

**Known limits.**
- A lane can reset the watchdog by committing anything, the Handoff included. The rule is "commit after every
  milestone", and a lane that commits a Handoff every 150 calls at least leaves a record.
- The budget is measured on the latest *finished* call, so it trips one call late.
- An `fc4-lane` agent spawned by hand (the Agent tool, no `lane:` label) is not guarded.
- Sessions read `.claude/settings.json` when they start. Start a new coordinator session in the main checkout after
  this lands, as with the agent types.

## Keeping the coordinator cheap

- **Restart fresh, from a note:** a call takes about 8 s under 50k tokens of context and about 60 s above 300k
  (measured on the Codex coordinator, whose context reached 446k). Every 10 trains, write a complete restart note in
  HANDOFF section 0 (live lanes and their rounds, the queue, the train, open alerts, the next three actions) so a
  fresh coordinator session can take over from it alone.
- **One monitor, events only:** run `python tools/lanes.py monitor` under the Monitor tool for the whole session. It
  prints a line only when a lane enqueues, a verdict is not clean, a train gate/regress finishes, `main` moves or a
  lane stalls; do not re-arm timers or poll status between events.
- **Keep the backlog launchable before a long wait (run 4, 2026-10-22):** before starting any wait longer than a few
  minutes (a train gate/regress above all), queue at least as many *launchable* briefs (every dep already on
  `main`) as there are open slots, and wait with `python tools/lane_heartbeat.py --wait-until train` instead of a
  plain train wait. It returns on the train verdict or as soon as the heartbeat says `BACKLOG LOW` (exit 4): then
  queue more launchable briefs and wait again. Briefs that depend on the train being waited for do not count; they
  drained run 4 from 9 live lanes to 1 while the log showed `pending 2`. See
  [docs/lane-tools.md](../lane-tools.md) "Run 4: the heartbeat backlog".
- **Read results, not transcripts:** the lane's structured result and `docs/lanes/<lane>.md` are the record; never
  read an agent's output file. Batch status updates to the user instead of one per event.
- **Measure:** `python tools/lanes.py ledger --md docs/history/token-ledger.md` after each landed batch; it reads the
  workflow journals and puts tokens per lane, merge share and tokens per coverage point in
  [docs/history/token-ledger.md](../history/token-ledger.md). Quote it, not estimates.

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
- **Gate.** The train's `gate` is `.claude/lane-bin/fc4verify.py gate` on the integration tree: clippy
  `-D warnings`, the CMake build, the full ctest (cargo_test included), each judged by parsing its output, never an
  exit code. The record is `build-runs/gate-<id>.json`; `land` and `regress` judge it again for the exact head they
  use (see "Build cache, lane checks and the gate" below). By hand, for one worktree:
  `python .claude/lane-bin/fc4verify.py --wt <worktree> gate`. Then `rustfmt --check --edition 2021` on
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
  latest-run line and `docs/manual-checks.md`; questions: see docs/specs/questions/README.md; regenerate `docs/coverage-report.md` and `docs/coverage-map.svg`
  (`python tools/coverage.py`, `python tools/coverage_treemap.py`) and check the delta: it must never go down.
- **Pause / stop:** `TaskStop`, resume with `SendMessage` to the lane's agent ID (but never `SendMessage` a
  workflow subagent: that spawns a duplicate). After a hard stop, check the worktree's *unstaged* diff
  (`git -C .claude/worktrees/<lane> status`): the agent may have been writing the fix to something it had just
  committed.

## Merge-train operator notes

Learned the hard way on 2026-10-16:

- `build-runs/coord-verify-lane.ps1` **exits 0 even when cmake fails**. Never take a gate from the wrapper's exit
  code. The train no longer calls it (fc4verify parses ninja and ctest itself); lanes may still use it by hand.
- **A gate is never set by hand.** Editing `state.json` to `gate: {ok: true}` is what landed train10's broken main;
  `land` now ignores that flag. If a gate failure is an expected change (a test pin that moved), commit the fix to
  the integration branch and gate again.
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
  `engine/target` and `engine/build-x86` of **merged, clean, 0-commits-ahead** worktrees only. Remove whole
  worktrees per [rules.md](rules.md) "Worktrees".

Learned 2026-10-20 and 2026-10-21:

- Union-glue land mines (moved from the 2026-10-20 HANDOFF):
  - The resolver's unions can swallow the ABI history's closing `*/`; check that the line before
    `#define FC4_ABI_VERSION` ends with `*/`.
  - They can duplicate `fc4.h`'s asserts tail; diff the tail against main's before gating.
  - Two lanes can take the same seam op; grep `kFC4Seam.* = n` for duplicates.
  - `#include` pairs, runner exec chains and switch statements can glue onto comment lines, and conformance modules
    can duplicate.
  - The gate must run with the worktree checked out at the batch's head.
  - A flaky `plugin_autotest_test` under parallel ctest passes when run alone.
- **`docs/abi/interfaces.tsv` is not docs-only.** CMake generates `abi_conformance_test` from it
  (`engine/CMakeLists.txt`, `gen_abi_entries.py`). Gate every change to it, including ABI-row batches committed
  straight to main: train18 and `ce3d44c0` were not gated, and a recovered row later broke the test.
- **Applying a lane's ABI rows:** match on the key (interface, game_class, vtable, slot). Add only keys that main lacks.
  A row whose key main already has is a judgment call: the proposal is not always the newer answer (`c1689949` undid
  an older proposal applied over a later one). Disagreeing proposals wait in `docs/abi/unreconciled-rows.tsv`. Lanes
  leave rows in `build-runs/pending-abi/` or `build-runs/abi-rows-*.tsv`, which git ignores: apply them when the lane
  lands, or they are lost with the worktree. 145 rows from 2026-10-15/16 sat there until `91854dbb`.
- **A lane that makes calls but writes nothing is exploring, not working.** re-answers35 made 2,789 calls for about $16
  with no output (REA was down). Now enforced: the lane guard's watchdog stops a lane after `maxCalls` calls without a
  commit, and RE lanes do not start while REA is unreachable ("Lane guard" above).
- **A wind-down order must survive compaction.** Run 2's coordinator wrote the user's "no more new work" into the
  HANDOFF, then started three lanes after its context was compacted (it cited a ">5 agents" target). The stop rule
  outranks any parallelism target in any document. Now a file, not a memory: `python tools/lanes.py stop "<reason>"`
  makes the workflow refuse every round until `lanes.py go`.

## Build cache, lane checks and the gate

Added 2026-10-20 after the day's audit (lanes: ~5 h in tools, 1.7 h of it builds/verify; train10 landed a main that
did not compile).

**sccache.** Install once per machine: `cargo install sccache --version 0.18.0 --locked` (5 min; lands in
`~/.cargo/bin`). `engine/CMakeLists.txt` (`FC4_COMPILER_CACHE`, default ON) then compiles C++ through it and passes
`RUSTC_WRAPPER` to its cargo commands; without sccache it builds as before. MSVC debug info is `/Z7` (cacheable
objects; the compiler no longer uses mspdbsrv, the linker still writes the PDB; `_MSPDBSRV_ENDPOINT_` stays private
per worktree/gate). `.claude/lane-bin/fc4verify.py` runs one sccache server per worktree (port from the path,
`SCCACHE_BASEDIRS` = the worktree) on one shared cache, `%LOCALAPPDATA%\fc4\sccache` (20 GB cap; disk is tight,
`SCCACHE_CACHE_SIZE` lowers it). Limits, measured: C++ objects hit across worktrees (157/157 in a fresh one); Rust
does not (sccache hashes rustc's working directory, and test binaries are linked, so never cacheable), so a fresh
worktree still compiles the Rust release library (~55 s). Objects from the cache carry the first worktree's paths
in their debug info and `__FILE__` strings.

**Lane checks.** `python <MAIN>/.claude/lane-bin/fc4verify.py --wt <WT> quick`: `cargo test -p` the crates changed
since `main` (the crates that depend on them are compiled with `--no-run`), then the CMake build. Before enqueueing:
`... build ctest` (the full ctest, `-j 4`; the DLL-loading tests are RUN_SERIAL). `cxxtest` is ctest without
cargo_test. The engine lane brief names both. `qbuild.py quick` runs `fc4verify.py quick`; `qbuild.py verify` runs `fc4verify.py build ctest`.

**Gate and land.** `gate` runs every step even after a failure and writes the record with the head, the tree and
its dirty files. `land` refuses unless the record is for the integration head, built a clean tree, and passes when
its step logs are parsed again; it also refuses when `main` changed build files since the batch was cut. `regress`
submits only the DLL the record names (sha256). `bisect` gates the halves one after the other, judged by records.
Regression tests: `python -m unittest discover -s tools/tests` (train10's cases, from real logs).

What let train10 through (records in the main checkout's `build-runs/`): `gate-train10.out` is a FAIL in
`cargo test` (the automata sweep's stale-index pin, 14 -> 0), so the C++ build never ran; `state.json` then shows
`gate: {ok: true, note: "cargo test + clippy pass (run 10); conformance via coord-verify-lane"}`, set by hand with
no gate log behind it; the regress passed on the integration worktree's `FreeCity4.dll` from 15:21, before the
17:08 merge; `land` read only `gate.ok`. The two compile breaks (fc4.h, FC4Subsystems.cpp) came from the
operator-resolved merge, so the lane's own green verify could not see them. The old `bisect` could not have helped
either: it reset the one integration worktree under both halves' parallel builds and read coord-verify's exit code.

Measured on this machine (20 threads), 2026-10-20, before -> after:

| | before | after |
|---|---|---|
| fresh worktree, build + full ctest | 304 s (coord-verify-lane.ps1: build 60, ctest 240) | 168 s (build 57, ctest 111) |
| merge-train gate, fresh tree | 444 s (cargo test 104, clippy 22, coord-verify 318) | 193 s (clippy 20, build 57, ctest 111) |
| lane loop after a one-line `engine/core` edit | 240 s (coord-verify-lane.ps1) | 107 s (`quick`: core tests, dependents compiled, build) |
| lane loop after a one-line plugin C++ edit | 172 s (coord-verify-lane.ps1) | 3 s (`quick`); 47 s with `cxxtest` |
| abi_conformance_test | 84 s | 9 s (no regex scan of files without the method name) |

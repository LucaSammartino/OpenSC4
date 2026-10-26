# Coordinating lanes

Coordinator manual: launching, resuming and landing lanes. Lane lifecycle rules belong to the
[prompt](../../.claude/workflows/fc4-lane.js); record formats belong to [lanes/README.md](../lanes/README.md).

## Spawning a lane

Use `tools/lane_launch.py` / `tools/lane_refill.py --provider claude|codex` with the shared ledger.
[lane-providers.md](lane-providers.md) owns authentication, model selection and provider recovery.

Every lane is one run of the saved workflow [`.claude/workflows/fc4-lane.js`](../../.claude/workflows/fc4-lane.js):
`Workflow({name: 'fc4-lane', args: {kind, name, task, date, brief, deps, game, fix?, model?, effort?, ctxBudget?,
maxCalls?, maxRounds?}})` (see "Lane context" and "Lane guard" below). It has no
merge step. Before each spawn ([tool reference](../lane-tools.md)):

0. No STOP flag (see "Lane guard").
1. `python tools/lanes.py slots --game-slots 6` must say `spawn ok`. If the shadows profile drops below
   3.0 game days/s, lower concurrency.
2. `python tools/lanes.py check <lane> --deps a,b --touches key1,key2 [--kind re]` must say `ok`: no STOP flag, the
   lane not parked, the work not assigned to an outside contributor ([docs/assignments.tsv](../assignments.tsv)), for RE lanes (`--kind re`, or any lane named `re-*`) a reachable REA server, dependencies landed on main
   (spawn in order core, then its DIFFERS/answers, then glue), no existing branch or lane file, no other active lane
   on the same keys. Do not spawn speculatively near a
   planned stop: a lane stopped midway is mostly wasted.
3. `python tools/lanes.py brief <subsystem> --kind engine|re --focus <regex>`, pasted as `args.brief`: the spec with its
   section outline (`--focus` marks task sections), questions file, core paths, keys, ABI and conformance pointers.
   Send only the files/sections needed; respect the prompt's do-not-read list.
4. Pick the model: `kind: 'mech'` (docs rows, ABI rows, regress records, regenerations) runs on Sonnet by default;
   RE, spec and engine work keep the session model. Pass `effort: 'low'` for mechanical steps.

Keep static prompt blocks byte-identical within each kind/provider for caching; lane-specific fields go last.
A fix round sets `fix` to the train's failure text. Keep the [game queue](running-the-game.md#the-game-queue)
running. Copy required external evidence into a lane's `build-runs/` when its transport cannot read it;
RE lanes use the main checkout's sources as their prompt directs.
Codex lanes cannot run the native build in their sandbox (MSVC configure hangs), so the coordinator also keeps
`tools/build_queue.py --jobs 1` running detached from the main checkout; `qbuild.py quick|verify` hands the native steps to it
while `build-runs/build-queue-worker.alive` is fresh, and it waits while a game run is in progress (`--status` shows the queue).

## Lane context

Claude loads [agent types](../../.claude/agents/fc4-lane.md) and settings once at session start; restart the
coordinator after changing them. RE uses [fc4-lane-re](../../.claude/agents/fc4-lane-re.md).
Do not use an unguarded fallback when an agent type is unavailable.

The workflow defaults are `ctxBudget: 150000`, `maxCalls: 150`, `maxRounds: 3`; the launcher reserves one
round per invocation and tracks the cumulative cap. A `continue` or null result starts a fresh round from
committed state plus the prompt's RESUME block; two consecutive nulls stop the loop. Labels are
`lane:<lane>` / `lane:<lane>#<n>`. The prompt owns milestone commits and the final/handoff lane file.
Claude measures latest input through `ctx.py`; Codex uses `codexguard.py` and its manifest.

Codex transport errors resume the same session after 60, 180, then 600 seconds (three retries maximum);
other errors fail. `codex-status.json` records retries; usage sums all attempts. Resumes retain permissions,
sandbox, model/effort, MCP, schema/output and `--ignore-user-config`; SessionStart refreshes the registered
session's transcript and refuses a new session ID.

Use the prompt's quiet wrappers; full build logs live in `<WT>/build-runs/qbuild-<step>.log`.
RE query paging and limits are in [re-workspace.md](../re-workspace.md#querying-in-parallel-lanes).

## Lane guard

The guard has two transport adapters: Claude's hook below and Codex's
`.claude/lane-bin/codexguard.py`. Both update the same per-lane counters and round/parking ledger.
Codex's first read-only identity probe initializes measured usage; missing later telemetry blocks work.
Provider-specific enforcement and recovery are in [lane-providers.md](lane-providers.md#guard-and-token-accounting).

Codex rounds default to 40 minutes (`args.roundMinutes` overrides), excluding retry waits. Five minutes before
the budget, only git, file reads and the lane's merge-queue write are allowed; five minutes after it, all tools
are denied and the backend terminates Codex, preserving commits and returning `continue`.
Set `codex.service_tier` or `args.serviceTier` (takes precedence) to `fast` or `default`; other values are rejected.

**The hook.** [`.claude/settings.json`](../../.claude/settings.json) runs
[`.claude/lane-bin/laneguard.py`](../../.claude/lane-bin/laneguard.py) `hook` before every tool call of every session
in the checkout. It acts only on agents of type `fc4-lane` / `fc4-lane-re` whose workflow label is `lane:<lane>` or
`lane:<lane>#<n>`; everything else (the coordinator, the gate agent, other subagents) passes straight through on a
fast path. Use settings hooks: project-agent frontmatter hooks can be skipped in untrusted `-p` sessions.
Hook input carries `agent_id` and `agent_type`; `transcript_path` is the parent transcript.
The guard finds the lane's own transcript and label beside it:
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

**Tests:** `python -m pytest tools/tests -q`; guard fixtures cover denials, permitted handoffs and workflow preflight.

**Known limits.**
- The budget is measured on the latest *finished* call, so it trips one call late.
- An `fc4-lane` agent spawned by hand (the Agent tool, no `lane:` label) is not guarded.

## Keeping the coordinator cheap

- **Restart fresh, from a note:** every 10 trains, write a complete restart note in
  HANDOFF section 0 (live lanes and their rounds, the queue, the train, open alerts, the next three actions) so a
  fresh coordinator session can take over from it alone.
- **One monitor, events only:** run `python tools/lanes.py monitor` under the Monitor tool for the whole session. It
  prints a line only when a lane enqueues, a verdict is not clean, a train gate/regress finishes, `main` moves or a
  lane stalls; do not re-arm timers or poll status between events.
- **Keep the backlog launchable before a long wait:** before starting any wait longer than a few
  minutes (a train gate/regress above all), queue at least as many *launchable* briefs (every dep already on
  `main`) as there are open slots, and wait with `python tools/lane_heartbeat.py --wait-until train` instead of a
  plain train wait. It returns on the train verdict or as soon as the heartbeat says `BACKLOG LOW` (exit 4): then
  queue more launchable briefs and wait again. Briefs depending on the current train do not count.
  See [heartbeat waits](../lane-tools.md#heartbeat-backlog-and-wait-until).
- **Read results, not transcripts:** the lane's structured result and `docs/lanes/<lane>.md` are the record; never
  read an agent's output file. Batch status updates to the user instead of one per event.
- **Measure:** `python tools/lanes.py ledger --md docs/history/token-ledger.md` after each landed batch; it reads the
  workflow journals and puts tokens per lane, merge share and tokens per coverage point in
  [docs/history/token-ledger.md](../history/token-ledger.md). Quote it, not estimates.

## Landing work: the merge train

The contract is [docs/merge-train.md](../merge-train.md); the tool is `tools/merge-train.py`, run from the main
checkout on `main`. A finished lane writes `build-runs/merge-queue/<lane>.json` in its own worktree and stops.
At review, run `python tools/merge-train.py enqueue --lane <lane> --branch <branch> --tip <tip> --kind <kind> [--log <game log>]` to attach checked coverage `credit`; the train refuses ready entries without it.

```
python tools/merge-train.py status                  # queued lanes, and entries that are not really ready
python tools/merge-train.py plan --size 12          # the next batch
python tools/merge-train.py run --size 12           # merge-batch -> gate -> regress
python tools/merge-train.py merge-batch --size 12   # merge onto integration/<n> in .claude/worktrees/train-integration
python tools/merge-train.py gate                    # once per batch
python tools/merge-train.py regress                 # once per batch (engine lanes)
python tools/merge-train.py land                    # fast-forward main; notices in build-runs/merge-notices/
python tools/merge-train.py bisect | fail --lane L  # a red batch
```

`merge-train.py regress` summarises logs with main's `tools/run_summary.ps1` and `tools/regress.ps1`.
A lane changing shadow summaries must land before the batch whose regress should count them;
otherwise re-summarise that regress's logs with the landed script.

- **Verify every landing:** `git merge-base --is-ancestor <lane> main` for each reported lane; the state list
  alone is not proof of ancestry.
- **Gate.** The train's `gate` is `.claude/lane-bin/fc4verify.py gate` on the integration tree: clippy
  `-D warnings`, the CMake build, the full ctest (cargo_test included), each judged by parsing its output, never an
  exit code. The record is `build-runs/gate-<id>.json`; `land` and `regress` judge it again for the exact head they
  use (see "Build cache, lane checks and the gate" below). By hand, for one worktree:
  `python .claude/lane-bin/fc4verify.py --wt <worktree> gate`. Then `rustfmt --check --edition 2021` on
  the touched `.rs` files from `engine/`.
- **Conflicts the train refuses** go to the operator (see the notes below). Resolve in the integration worktree with
  `git -C <wt> -c submodule.recurse=false ...` and add files explicitly (`commit -a` fails on the worktree's
  submodule pointer). Renumber seam ops / switch counts there if two lanes took the same one. The ABI version is
  bumped once per batch by `python tools/bump_abi.py --batch <batch-id> --lanes laneA,laneB` (the train calls it;
  never per lane, never by hand). `abi_entries.inc` is generated at build time and not in git: never merge,
  renumber or revert it. Write merge messages to a file and use `-F <msgfile>`; preserve the lane prompt's
  provider attribution and contributor credit.
- **After a landing:** the regress ran in the batch (read the shadow column, not only the verdict); run records go
  in `docs/runs/<tag>.md` and `python tools/gen_handoff_tables.py` regenerates `docs/lanes/INDEX.md`, the HANDOFF's
  latest-run line and `docs/manual-checks.md`; questions: see docs/specs/questions/README.md; regenerate `docs/coverage-report.md` and `docs/coverage-map.svg`
  (`python tools/coverage.py`, `python tools/coverage_treemap.py`) and check the delta: it must never go down.
- **Pause / stop:** `TaskStop`, resume with `SendMessage` to the lane's agent ID (but never `SendMessage` a
  workflow subagent: that spawns a duplicate). After a hard stop, check the worktree's *unstaged* diff
  (`git -C .claude/worktrees/<lane> status`): the agent may have been writing the fix to something it had just
  committed.

### Coverage credit before ready

When tasked to edit `tools/coverage-map.tsv`, lanes change only their assigned owners' rows.
Spec paths, Rust core paths, shadow keys and replace keys must be justified by the lane tree.
Run `python tools/creditcheck.py --wt <worktree> --log <game.log>` before ready; repeat `--log` for more logs.
Only exact shadows present in that tree's regress shadows profile earn verified credit.
Use `--json <out>` to retain rows flipped, byte deltas and refusals. The review enqueue checks the claim;
refusals block ready. `merge-train.py plan` prints claims; `land` prints coverage before and after the batch.

## Merge-train operator notes

- **A gate is never set by hand.** `land` ignores `state.json`'s `gate.ok` and reparses gate records/logs.
  For an expected test change, commit the fix to integration and gate again. Do not use
  `build-runs/coord-verify-lane.ps1`'s exit code as a gate; it can exit 0 after a failed cmake.
- Lanes append to shared files (`fc4.h`'s ABI history, `FC4Config.*`'s key table, `FC4Subsystems.cpp`'s registry),
  and **each side's addition ends with the file's own closing `}`/`*/`**. The resolver now splits that close off the
  last line, commas list entries only, checks the three-way brace balance, and rejects glued `fn` signatures. It also
  refuses changes outside `fc4.h`'s history block and collapses a duplicated `FC4_ABI_VERSION`.
- `git checkout --theirs -- fc4.h` during a manual resolve is dangerous: the lane's copy is **stale** (cut from an
  older main) and drops everything merged since. Restore from the consolidate commit instead.
- `reset_integration` (`checkout --force -B integration/<n> main`) discards a previous attempt's merges, so a batch
  re-run from scratch can record a lane as merged whose merge did not survive; `land` writes its list from its own
  state file, which is not a proof of ancestry.
- `docs/HANDOFF.md` and `docs/lanes/INDEX.md` conflicts take the integration side, then regenerate with
  `tools/gen_handoff_tables.py`. Union `docs/manual-checks.md`, then regenerate its run table.
- `docs/specs/QUESTIONS.md` conflicts take either side, then `python tools/questions.py index`.
  Open question files can union two append-only changes; other conflicts require operator review.
- `fc4.h` and `engine/capi/src/lib.rs` version conflicts take the higher constant; other hunks need normal review.
- PowerShell `*>` output in `build-runs/` is UTF-16; `tr -d '\0'` before grepping.
- A git worktree's `engine/third_party/gzcom-dll/.git` holds a relative `gitdir:` that does not resolve — repoint it
  at `<main>/.git/modules/engine/third_party/gzcom-dll` or every git command in that worktree fails.
- Disk: build caches are ~10-19 GB per worktree. Free
  `engine/target` and `engine/build-x86` of **merged, clean, 0-commits-ahead** worktrees only. Remove whole
  worktrees per [rules.md](rules.md) "Worktrees".

- Union-glue checks:
  - The resolver's unions can swallow the ABI history's closing `*/`; check that the line before
    `#define FC4_ABI_VERSION` ends with `*/`.
  - They can duplicate `fc4.h`'s asserts tail; diff the tail against main's before gating.
  - Two lanes can take the same seam op; grep `kFC4Seam.* = n` for duplicates.
  - `#include` pairs, runner exec chains and switch statements can glue onto comment lines, and conformance modules
    can duplicate.
  - The gate must run with the worktree checked out at the batch's head.
  - A flaky `plugin_autotest_test` under parallel ctest passes when run alone.
- **`docs/abi/interfaces.tsv` is not docs-only.** CMake generates `abi_conformance_test` from it
  (`engine/CMakeLists.txt`, `gen_abi_entries.py`). Gate every change, including coordinator ABI-row batches.
- **Applying a lane's ABI rows:** match on the key (interface, game_class, vtable, slot). Add only keys that main lacks.
  Existing keys require review: a proposal is not necessarily newer. Disagreements wait in `docs/abi/unreconciled-rows.tsv`. Lanes
  leave rows in `build-runs/pending-abi/` or `build-runs/abi-rows-*.tsv`, which git ignores: apply them when the lane
  lands, before worktree cleanup loses them.

## Build cache, lane checks and the gate

**sccache.** Install once per machine: `cargo install sccache --version 0.18.0 --locked` (5 min; lands in
`~/.cargo/bin`). `engine/CMakeLists.txt` (`FC4_COMPILER_CACHE`, default ON) then compiles C++ through it and passes
`RUSTC_WRAPPER` to its cargo commands; without sccache it builds as before. MSVC debug info is `/Z7` (cacheable
objects; the compiler no longer uses mspdbsrv, the linker still writes the PDB; `_MSPDBSRV_ENDPOINT_` stays private
per worktree/gate). `.claude/lane-bin/fc4verify.py` runs one sccache server per worktree (port from the path,
`SCCACHE_BASEDIRS` = the worktree) on one shared cache, `%LOCALAPPDATA%\fc4\sccache` (20 GB cap; disk is tight,
`SCCACHE_CACHE_SIZE` lowers it). C++ objects hit across worktrees; Rust hashes the working directory and test
binaries are linked, so fresh worktrees still compile Rust. Cached objects carry the first worktree's paths
in their debug info and `__FILE__` strings.

**Lane checks.** The prompt owns the final `qbuild.py all` cadence. During development, `qbuild.py quick`
runs `fc4verify.py quick`: `cargo test -p` changed crates, dependents with `--no-run`, then CMake.
`qbuild.py verify` maps to `fc4verify.py build ctest`; ctest uses `-j 4`, DLL-loading tests RUN_SERIAL.
`cxxtest` excludes cargo_test. These are focused diagnostic modes, not additional mandatory final gates.

**Gate and land.** `gate` runs every step even after a failure and writes the record with the head, the tree and
its dirty files. `land` refuses unless the record is for the integration head, built a clean tree, and passes when
its step logs are parsed again; it also refuses when `main` changed build files since the batch was cut. `regress`
submits only the DLL the record names (sha256). `bisect` gates the halves one after the other, judged by records.

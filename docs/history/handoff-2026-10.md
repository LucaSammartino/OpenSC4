# HANDOFF history, October 2026

Moved verbatim from `docs/HANDOFF.md` on 2026-10-20, when the HANDOFF was cut to one page. Nobody needs to
read this to start work: it is the record, newest first. The lane table that sat under "Wind-down 2026-10-14"
is generated into [docs/lanes/INDEX.md](../lanes/INDEX.md) now.

Section numbers inside (§0-§8) are the HANDOFF's as it was then. Where they went: §0 current state stays in
[docs/HANDOFF.md](../HANDOFF.md); §2 is [docs/ops/rules.md](../ops/rules.md), §3
[docs/ops/running-the-game.md](../ops/running-the-game.md), §5 [docs/ops/lessons.md](../ops/lessons.md) (same
numbering), §6 [docs/ops/coordinating.md](../ops/coordinating.md), §8
[docs/ops/conventions.md](../ops/conventions.md); §1, §4 and §7 are condensed in the HANDOFF.

## Stop notes

### Stop 2026-10-21 (wind-down after run 2): HANDOFF §0a, §0z and §0 as they stood

Moved verbatim on 2026-10-21 when the HANDOFF was cut back to its current state. Headings are two levels down.

#### 0a. Since the pause: efficiency work landed (2026-10-20 evening) - read before restarting lanes

The user wants to restart at full throttle on these changes. Why: the day's $184.71 OpenRouter bill was 85% agents
re-reading cached context (lanes started every call at 172k tokens and grew to 450-970k). See
[ops/efficiency-log.md](ops/efficiency-log.md).

**Landed on main:**
- **`3975a42e`:** main builds again (train10's two merge breaks). The automata sweep's pin is now 0, per spec §12
  quirk 10: Read keeps the streamed `+0x0C`. **This resolves train11's gate failure in item 1 below:** 14 -> 0 is
  the correct result. Re-cut integration/11 on main and re-gate it.
- **Lean lanes (`0e1ebcca`):**
  - Agent types `fc4-lane` and `fc4-lane-re` with a minimal tool list (probe: first call 7.4k tokens vs 39k for a
    default subagent, subscription route).
  - Stateless lanes: a `## Handoff` section in docs/lanes/<lane>.md, a restart in a fresh agent past 150k context
    or after an API error.
  - Quiet wrappers in `.claude/lane-bin/`: `qbuild`, `qgrep`, `qwait` (one blocking wait, no polling).
- **Forge (`b8628bfe`):** `tools/forge.py` writes a class from its spec in a scripted loop with no agent. The
  pilot did History, Parks and Dispatch for $0.41 in total; History and Parks passed their class tests, and Parks
  ran clean in game. See [ops/forge-pilot.md](ops/forge-pilot.md). Use it for classes that have class tests.
  Backends: `openrouter` (key in `%USERPROFILE%\.config\fc4-forge\openrouter.key`, a test key) or `claude-code`
  (the user's subscription through the `claude` CLI, about 1.3k tokens of overhead).
- **Build and gate (`445ed0ac`, `4c601fac`):**
  - sccache for C++ and Rust, /Z7 debug info. A fresh-worktree build + ctest went from 304 s to 168 s.
  - `fc4verify.py quick` for lane checks, reached as `qbuild.py quick`; a one-line plugin edit is checked in 3 s.
  - The train's `land` now requires a parsed post-merge gate record for the exact head. A hand-set `gate.ok` no
    longer lands anything.
  - `engine/tests/abi_entries.inc` is untracked again.

**Restart checklist:**
1. **Start a NEW coordinator session in the main checkout.** Agent types load only at session start; an old
   session's lanes fall back to every tool and 172k context (fc4-lane.js logs it).
2. **Check the first lane's first-call context** in its transcript, or with `.claude/lane-bin/ctx.py`. On the GLM
   route it should be far below 172k. If it is still near 172k, turn off the Chrome, computer-use and browser
   connectors for the coordinator session (docs/ops/coordinating.md "Lane context").
3. **Re-gate train11** on the new main (item 1 below); `land` refuses without the gate record.
4. **Track the day:** `python tools/spend.py <openrouter csv> --coverage-delta <pts>` adds a row to
   ops/efficiency-log.md. The baseline is $46 per implemented point.
5. The "no new lanes" rule (item 5 below) was the pause rule. The user said to restart at full throttle once
   back: confirm with them, then fill the slots.

**Restart state (2026-10-20 ~21:45, new coordinator session in the main checkout):** checklist done: new session
started; first-call context of the first lanes measured **7.8k tokens** (`ctx.py glue21fix2 --first`), the lean
harness works; the "no new lanes" rule is lifted by the user's restart order (full throttle). Done: re-answers34's
stale tip fixed (queue file now points at `0eb16695`), train10 re-cut as **cores27 only** (batch seq 10;
`3975a42e`'s automata-pin fix made the old train11 gate failure moot; glue21 re-queues when PL17 is fixed) —
gate PASS (clippy, build, ctest), regress running. Spawned 8 lanes: glue21fix2 (PL17, on the glue21 branch),
glue24fix2 (the tick-5500 int3, on the glue24 branch), re-netocc (the network occupants spec, 754 fns —
re-specs22's work is NOT recoverable: its branch had no commits and no queue file; re-do it), re-terrain3d
(TR21, terrain.md §15.5), devperf2 (UpdatePopulation 7.7x → target 2x), re-occprops (specify the occupants and
props pool, 453 fns), cores28 (MySimPool + MySimDispatch cores), glue29 (the cSC4NetworkTool replacement glue
over cores27's fc4_nettool ABI). Pending after train10 lands: land, gen_handoff_tables, coverage, push;
re-answers34 merges in the next batch.

#### 0z. WIND-DOWN (2026-10-20 ~23:00): the user said "no more new work creation, winding down only"

No new lanes spawn from here. The 8 lanes still running (glue21fix2, re-terrain3d, devperf2, cores28, glue29,
re-occprops, re-answers35, mysims1) finish their current task; when each enqueues, the coordinator merges,
gates, regresses and lands it as usual, then stops. Landed since the restart: **train10** (cores27, the
network-tools core, ABI 75), **train11** (re-answers34, effects EF1-EF10), **train12** (glue24fix2: the
network managers glue `enable = netmgr,netlot` with the tick-5500 int3 DISARMED — a Debug build compiles
/RTC1 with no RTC error func; the handler is now installed in every build; a/b/save all clean), **train13**
(re-netocc: the network-occupants spec, 14 classes) + the coverage map wired → **specified 35.0%**
(32.9 → 35.0), implemented 25.3%, verified 10.5%. Main is `98662e3b`, pushed. Lane states at wind-down
(see docs/lanes/<lane>.md for each): glue21fix2 mid-PL17 (slot 27 GetObjectIDArray fix landed on the branch;
isolation runs at `exited -1`, gate runs corrupt — diagnosing, its queue entry stays `wip`), devperf2
measured UP 3.52 us/call at baseline and its fix3 run clean (63.96 days/s), cores28/glue29/re-terrain3d/
re-occprops/re-answers35/mysims1 in progress. The next coordinator: merge whatever enqueues, land, stop;
do NOT spawn.

#### 0. Current state: pause 2026-10-20 ~19:10 (user closing the session)

Main is `12e79cfc`, pushed. Today landed: train6 (8 lanes), train8 (6), train9 (cores26), train10 (glue19),
re-answers27/32/33 doc merges, 25 measured ABI rows applied to interfaces.tsv (the sdk-errata slot-14 x87 claim
is wrong), and ~10 union-glue land mines fixed (see the operator notes below). Coverage: **implemented 25.3%,
verified 10.5%** (from 21.3/9.5 this morning). Token ledger: docs/token-ledger.md.

**Open state to resume (in this order):**

1. **train11 is mid-batch**: integration/11 (a23223be) = glue21's refcount fixes merged (operator-resolved),
   built on main's tip. Its gate FAILED once on the stale-`+0x0C` corpus count (14 -> 0,
   `formats/tests/automata_records.rs` asserts 14) - the same failure train10's first gate had; it was a
   real ordering difference there. Diagnose before landing: either glue21's branch regressed the
   renormalisation path or the merged tree mixed versions. State: current=train11, head a23223be,
   gate ok=False.
2. **glue21's lane result arrived** (tip `1c637e99`): the refcount chain fixed (QI AddRefs, the creation
   reference handed over, slot 20/21 rebuilt); its gate-11 run found a NEW fault (PL17, the game's own
   CreateLotObjects AV at SimCity 4.exe+0x60D4B5, ecx=0) - filed in
   docs/specs/questions/prop-lotconfig.md. Its queue entry (build-runs/merge-queue/glue21.json in the
   worktree) is still `wip`: PL17 must be fixed or accepted-as-question before it lands.
3. **glue24** (tip `b2e676f9`): runs clean to 2032 (tools=13 configs=543) and the save round trip clean;
   one intermittent debug-CRT int3 at tick ~5500 in 3 of 5 runs, no stack. Two fix rounds died on API
   errors mid-chase (a symbolizer tool is in its worktree's build-runs/). Resume the fix round or
   accept NM11 as a filed question and land as-is with the int3 documented.
4. **Finished, waiting to land (paid, not counted in coverage yet)**: re-specs22 (the network-occupants
   spec, 754 fns), cores27 (the network-tools core, ABI 75 - its queue entry says the train bumps the ABI
   once and takes history line 75), re-answers34 (EF1-EF10). Check each worktree's
   build-runs/merge-queue/*.json and merge through the train.
5. **No new lanes** unless the user says so; glue28 was halted (zero output, worktree clean at main's tip).
6. After every landing: `gen_handoff_tables.py`, `coverage.py` + `coverage_treemap.py`, push; check
   `git merge-base --is-ancestor <lane> main` for every lane the train lists as landed.

**Operator notes for union glue** (this session's list, all found the hard way): the resolver's unions can
swallow the ABI history's closing `*/` (check the line before `#define FC4_ABI_VERSION` ends with `*/`);
duplicate fc4.h's asserts tail (diff the tail against main's before gating); two lanes can take the same
seam op (grep `kFC4Seam.* = n` for duplicates); `#include` pairs, runner exec chains, and switch statements
can glue onto comment lines; conformance modules can duplicate; the gate must run with the worktree checked
out at the batch's head. A flaky plugin_autotest_test under parallel ctest passes standalone. A lane whose
transcript grows but writes no files for 45+ min is exploring, not working - kill it.

**Coverage** (`python tools/coverage.py`, on main): 29,481 functions / 5,586,418 bytes; **32.9% specified /
25.3% implemented / 10.5% verified / 12.1% replaced**. Never let it go down.

**Open items, ranked:**
1. `enable = automata` - LANDED (train10); the riot/fire host tables and U-Drive-It are wired; AT20 filed.
2. `enable = propmgr`/`lotcfg` (glue21): refcounts fixed, PL17 open, then lands.
3. `enable = netmgr`/`netlot` (glue24): clean runs + save round trip done; the int3 question open.
4. `enable = terrain`: needs TR21's terrain 3D view contract (`terrain.md` §15.5).
5. Performance: glue27 measured the development glue at 0.75x the original's cost per game day; UpdatePopulation
   is still 7.7x (glue27's report).

### Stop 2026-10-16 ~21:05 (user halted): how to resume

**The merge train is built and working** — `tools/merge-train.py`, contract in [docs/merge-train.md](../merge-train.md).
`build-runs/main-merge.lock` is gone. A finished lane writes `build-runs/merge-queue/<lane>.json` in its own worktree
and stops; the coordinator is the only writer of `main`. Four batches landed this session.

**Resume in this order:**

1. **Land batch `train5`.** It is merged on `integration/5` in `.claude/worktrees/train-integration`, its **gate is
   PASS** (`build exit=0`, ctest 58/58) but its **regress was never run**. From the main checkout:
   `python tools/merge-train.py regress` then `python tools/merge-train.py land`. Lanes in it: `re-answers31`,
   `re-specs21`, `cores24` (audio core), `coverage3` (the coverage map), `glue23` (demolition host).
   **`coverage3` is the one that moves the coverage number** — its map rows take `implemented` 21.3% → 23.1% and
   `verified` 9.5% → 10.5%; those figures are on its branch, not on `main` yet.
2. **Then the queue.** `python tools/merge-train.py status` / `plan` / `merge-batch --size 12` → `gate` → `regress` →
   `land`. Ready now: `cores20` (bridges), `cores23` (UI windows), `cores25` (effects manager), `re-answers27`,
   `glue18`, `glue22`. On hold with uncommitted edits: `glue19`, `glue21`, `glue24`, `re-answers27` — check
   `git -C .claude/worktrees/<lane> status` before resuming each.
3. **Four lanes could not be merged by the train** and need an operator pass: `glue22`, `glue23`, `glue18`, `cores20`
   conflicted on `engine/capi/include/fc4.h` / `lib.rs` / `FC4Subsystems.cpp` / `terrain/water.rs`. Their unions were
   **refused rather than mangled** (see the guards below) — resolve by hand and verify
   `git diff main -- engine/capi/include/fc4.h` shows no line of main's header disappearing.

**What landed on `main` this session: 29 lanes** (`train1` 4 docs, `train2` 12, `train3` partly, `train4` 7).
**CORRECTION found after the halt:** eight lanes the merge train reported as landed in `train3` are **NOT on
`main`**. Their work is safe and committed on their own branches (worktrees clean), but
`git merge-base --is-ancestor <lane> main` is false for each:

| not on main | what it carries |
|---|---|
| `cores17` | the **network geometry** core (`fc4_netgeom`) |
| `cores19` | the **effects** core |
| `glue12b1` | prop developer + city detail glue |
| `glue12b2` | ports glue (airport/seaport developers) |
| `glue15` | `enable = tutorial`, the three exact prop/lotcfg/tutorial shadows |
| `glue17` | citysit/demolition/dispatch shadows + `enable = citysit`/`dispatch` (+829/-1) |
| `apply15b` | PD4-PD10, PV8/12/13, CD4/5, RN3, HW2 |
| `apply15terrain` | the terrain DIFFERS answers (TR4/TR7/TR11/TR14/TR3) |

**Actually on `main`:** `re-answers20/23/26/28/29`, `re-specs19/20`, `re-answers30`, `re-integrate1`,
`mergetrain-docs`, `coverage2`, `cores12a`, `cores12c`, `cores21`, `cores22`, `detpacing`, `cores18`,
`apply14dis`, `apply14traffic`, `apply15c`, `apply15d`, `apply15e`, `glue14b`, `glue14c`, `flakyfix`,
`abi-hygiene`, `glue20`.
**Always verify with `git merge-base --is-ancestor <lane> main`. Never trust the train's own lane list.**

**Why:** `train3` was re-run from scratch several times while the resolver was being fixed, and
`reset_integration` (`checkout --force -B integration/<n> main`) discards a previous attempt's merges. A lane was
recorded as merged even where its merge had not survived, and `land` writes its list from its own state file,
which is not a proof of ancestry. **Do this before anything else:** merge those eight branches (gate + regress +
land), then re-run the ancestry check for every lane listed as landed.

Landed work worth noting: network-rules and network-bridges cores, demolition/dispatch/city-situation,
prop-occupant and audio cores, the disaster glue (fire 256/256, riot 352/352 exact in its lane run), `flakyfix`,
`abi-hygiene`, and 8 RE spec lanes (`ui-windows.md`, `audio.md`, `view3d.md`, `prop-occupant.md`, plus
automata/effects/terrain/disasters answers). Batch 2/3/4 regressions **PASS**.

**Coverage (measured, `python tools/coverage.py`):** game code 29,481 functions / 5,586,418 bytes;
**32.9% specified / 21.3% implemented / 9.5% verified / 12.1% replaced**; shared 206,377 B, unattributed 95,483 B.
Flat this session because the new subsystems had no `tools/coverage-map.tsv` rows — `coverage3` fixes that and is
queued. **Never let this number go down**; regenerate `docs/coverage-report.md` and `docs/coverage-map.svg` after
every merge and check the delta.

**Merge-train operator notes (learned the hard way this session):**
- `build-runs/coord-verify-lane.ps1` **exits 0 even when cmake fails**. Never take a gate from the wrapper's exit
  code — `merge-train.py`'s `gate` parses `build exit=`, `ctest exit=` and the ctest summary.
- Lanes append to shared files (`fc4.h`'s ABI history, `FC4Config.*`'s key table, `FC4Subsystems.cpp`'s registry),
  and **each side's addition ends with the file's own closing `}`/`*/`**. The resolver now splits that close off the
  last line, commas list entries only, checks the three-way brace balance, and rejects glued `fn` signatures. It also
  refuses to change anything outside `fc4.h`'s history block and collapses a duplicated `FC4_ABI_VERSION` — a bug
  there once deleted all 126 `FC4_ADV_SRC_*` defines and 308 declarations.
- `git checkout --theirs -- fc4.h` during a manual resolve is dangerous: the lane's copy is **stale** (cut from an
  older main) and drops everything merged since. Restore from the consolidate commit instead.
- PowerShell `*>` output in `build-runs/` is UTF-16; `tr -d '\0'` before grepping.
- A git worktree's `engine/third_party/gzcom-dll/.git` holds a relative `gitdir:` that does not resolve — repoint it
  at `<main>/.git/modules/engine/third_party/gzcom-dll` or every git command in that worktree fails.
- Disk: build caches are ~10-19 GB per worktree. `C:` hit ENOSPC twice mid-gate. Free
  `engine/target` and `engine/build-x86` of **merged, clean, 0-commits-ahead** worktrees only.

**Open items the next session should pick up (ranked):**
1. `enable = automata` — **re-answers31 found the root cause**: `QueryInterface` answers the CLSID but never the IID,
   so `cSC4City+0x1E4` (the automata script system, IID `0x8A1B6647`) stays null and the city's Init faults at
   `0x45BF1B`. Fix is to accept the IID and return the interface sub-object.
2. `enable = effects` — the manager is **six** sub-objects (vptrs at `+0x00 +0x04 +0x08 +0x0C +0x10 +0x94`); the
   fault is a dispatch on `+0x08` past a one-interface object.
3. `enable = lotcfg` — the configuration class is not GZCOM-creatable, but `FUN_00489910` **is** the named
   allocate-and-construct pair and the *manager* is GZCOM-creatable (CLSID `0x299062C0`). Drive the manager.
4. `enable = netmgr`/`netlot` (glue24, `wip`) — crashes at PostCityInit with `tools=0, configs=0`; wire `create_tool`
   and feed the 543 type-0x21 exemplars before `Init`.
5. `enable = terrain` — needs TR21's terrain 3D view contract (now in `terrain.md` §15.5); watch the two globals
   `view->Init` dereferences with no null check (`0x00B43CF0`, `0x00B43CD8`).
6. Performance: the development glue is ~3x slower per game day and the tract developer's `Grow` is 1.5-2.5x the
   original (glue27 is on it — measure, do not estimate).

### Stop 2026-10-16 ~09:10 (user shut down): how to resume

All lanes were stopped at once, without asking them to commit first. Main is clean at the last merge (re-answers24,
`7d06e75`); the stale `build-runs/main-merge.lock` was removed. Each stopped lane's committed work is on its branch,
and its worktree under `.claude/worktrees/` is kept. Uncommitted edits in a worktree may be partial: check
`git -C .claude/worktrees/<lane> status` before resuming. Every lane below had finished its own work and was waiting in
its merge step, except glue15, flakyfix and apply15b, which were stopped mid-lane. Resuming a lane means re-running its
merge: merge main, gate, regress for engine lanes, then `--no-ff` into main.

- Engine, with game runs: cores12a (revert the revert `dff8e4e`; re-apply seeding's `e451878` city-situation change),
  cores12c, devperf, cores17 (network geometry), apply14traffic, glue14b (occupant manager shadow), apply15terrain,
  detpacing (deterministic pacing default), glue12b1 (prop developer, city detail), glue12b2 (ports), glue14c (network
  manager shadow), apply14dis (disaster DIFFERS).
- Engine, core only: cores18 (network rules), cores19 (effects).
- RE, docs only: re-answers20 (AM1-AM12), re-answers23 (disasters DS25-DS40), re-answers26 (NB1-NB3), re-inventory6
  (INVENTORY-M6.md, the renderer/UI/audio scoping).
- Tools: coverage2 (attribute the 203 KB draw range, split catch-all owners, map the new cores).
- Stopped mid-lane: glue15 (prop manager, lot configuration, tutorial glue), flakyfix (the history/clock24 thread tests
  fail gates under load; fix these first), apply15b (re-answers22 DIFFERS: ports PD4-PD10, prop developer PV8/12/13,
  city detail CD4/5, seeding RN3).
- Not started: apply TG23-TG25 after apply14traffic; terrain replacement after apply15terrain (TR9 zoom, message
  latency); bridges core after cores17 and NB1; engine follow-ups PL12, PL13, TR20, TU5; held ABI rows in
  `build-runs/pending-abi/re-answers25-rows.tsv` (rebuild build-x86 at main's tip first).

Progress is measured with `python tools/coverage.py` and drawn with `python tools/coverage_treemap.py`
(`docs/coverage-report.md`, `docs/coverage-map.svg`). Quote those numbers, not estimates.

#### ~~First task: replace the merge lock with a merge train~~ — DONE 2026-10-16 (see the stop note above)

The merge of each lane is slow, not git. Measured on 2026-10-16:

- Each lane ran its own gate and regress (cargo, ctest, two game runs to 2032: 20-40 min on a loaded machine).
- Every merge into main forced each waiting lane to merge main again and re-gate. cores14c lost the lock 12 times and
  re-merged 11 times, so N waiting lanes cost about N² gates.
- Nearly every lane edits the same lines, so every merge conflicts and needs an agent:
  - the FC4_ABI_VERSION number;
  - the HANDOFF §0 rows;
  - the run numbers in manual-checks.md;
  - QUESTIONS.md sections;
  - abi_entries.inc (done: it is generated at build time and no longer in git).
- About 25 agents polled `build-runs/main-merge.lock`; the fastest poller won, so docs-only lanes waited about 2 h.

The plan:

1. **Merge queue with a single owner of main.** A lane that is ready writes `build-runs/merge-queue/<lane>.json`
   (branch, tip, kind, abi needs, game-run labels) and stops. One merge-train agent is the only writer of main.
   Lanes no longer take the lock or merge main.
2. **Batches with bisection.**
   1. The train takes up to K ready lanes (start with K = 8), oldest first.
   2. It merges them one after another onto `integration/<n>` from main, resolving conflicts once.
   3. It runs the gate and regress once for the whole batch.
   4. On a pass, main fast-forwards to the integration branch and each lane gets a merged notice.
   5. On a fail, it splits the batch in halves and gates both halves in parallel on separate game-queue slots, until
      the culprit lane is isolated (about log2 K extra gates). The culprit goes back to its lane with the failure, and
      the rest land.
   6. Docs-only and RE lanes skip regress and can be batched freely.
3. **Remove the shared conflict hotspots**, so most batches merge with no agent conflict resolution:
   - **ABI version.** Lanes never bump FC4_ABI_VERSION. The train bumps it once per batch with a script, and writes
     one fc4.h history line per batch listing the lanes. The script is `tools/bump_abi.py`
     (`python tools/bump_abi.py --batch <batch-id> --lanes laneA,laneB`); it also updates the mirrored Rust const in
     `engine/capi/src/lib.rs`, refuses a dirty `fc4.h` and refuses to run twice for one batch
     (`build-runs/abi-batches.json`). It does not commit and does not run git.
   - **Per-lane files.** Each lane writes `docs/lanes/<lane>.md` (today's HANDOFF §0 row) and `docs/runs/<tag>.md`
     (today's manual-checks rows, keyed by tag, not by a sequential run number). A script regenerates the HANDOFF §0
     table, the manual-checks index and the "latest in-game run" line from them. **Done** (mergetrain-docs):
     `tools/gen_handoff_tables.py`, with the row formats and the merge-queue contract in `docs/lanes/README.md`.
   - **abi_entries.inc** becomes a build-time generated file (CMake runs gen_abi_entries.py) and leaves git.
     **Done (abi-hygiene lane):** `engine/tests/abi_entries.inc` is gitignored and no longer tracked; the CMake
     target `fc4_abi_entries` runs `gen_abi_entries.py` on every build and `fc4_abi_conformance_test` depends on it,
     so a stale listing can never be compiled. With no Python 3 the conformance test is skipped, like
     `ret_channel_check`. `python tests/gen_abi_entries.py` from `engine/` still works standalone.
   - **QUESTIONS.md.** Use per-spec question files, or keep one file with a fixed "append at the end of your spec's
     section" rule.
   - **Docs-only movement on main** never triggers a re-gate (engine/, tools/ or tests/ unchanged means the gate
     result stands).
4. **Tooling.** Build `tools/merge-train.ps1` (or .py): queue scan, batch merge, gate, regress via the game queue,
   bisection, fast-forward, notices. Change `lane.js` so the merge step only enqueues. Update the lane prompts (MKWT,
   LOCK) and CONTRIBUTING if it mentions the lock.
5. **First batch.** Run it on the lanes listed above that were stopped while waiting to merge.

Expected effect: no lock waits, several times fewer regress runs, and most of the merge-time agent work (re-merging,
renumbering, conflict resolution) gone. The estimate for a standalone OpenSC4 was about 450-550 more lanes and
150-300M agent tokens, measured from about 400k tokens per lane. With the train, the CPU (20 threads) becomes the
limit instead of the merge, which takes that from about 3-4 weeks of continuous running to about 2-3. Also log each
lane's token count against the coverage bytes it moved, so tokens per coverage point becomes a measured figure.

### Wind-down 2026-10-14 ~16:20 (usage limit): how to resume

Every lane was asked to stop at a clean point, commit everything on its branch (`WIP:` commits where tests did not pass)
and report its state. Worktrees are under `.claude/worktrees/` and are kept (never delete them). To resume a lane,
`SendMessage` its agent ID if it is still reachable; otherwise start a fresh lane in the same worktree with the
original prompt from `build-runs/lane-prompts/<lane>-prompt.txt` plus the "next steps" below.

Lane state, one row per lane. The table is generated from `docs/lanes/<lane>.md` (the format and the
merge-queue contract are in [`docs/lanes/README.md`](../lanes/README.md)). Do not edit the table: edit the
lane's own file and run `python tools/gen_handoff_tables.py`.

*(2026-10-20: the generated lane table that followed here moved to [docs/lanes/INDEX.md](../lanes/INDEX.md).)*

## State lines as of 2026-10-15

*Last updated 2026-10-15, at the merge of traffic2 (`40d9eab`), after run 119.*

- **main:** `FC4_ABI_VERSION` 56 (traffic2 merge `40d9eab`). Last gate: traffic2, cargo 2048/0;
  clippy clean, ctest 55/55. Test switches `sim_turbo`, `autotest_end_date`, `script`, `close_dialogs` (engine/README). Every glue except the simulator and the budget is off by default (`enable = <key>`).
- **Milestones** (details in [PROGRESS.md](../PROGRESS.md)): M0, M1 and M2 done. **M3 exit met in run 103** (regress r6c): every grid
  shadow exact, including power 463/463, plumbing 179/179 and landfill 89/89, zero INCOMPLETE lines. M4: all cores
  merged; glue merged for residential, zones phase 1 and development (BDS + growth developers, opt-in), but the
  development glue fails in game (runs 101-102). M5: all three cores merged; traffic glue phase 1 and the traffic shadow (phase 2) merged; calibration open. M6: the
  landfill, neighbours, exemplar and formats work is merged.

The generated latest-run bullets as they stood on 2026-10-16 (the region keeps regenerating in the HANDOFF):

- **Latest in-game run:** 142 (regress apply15c, clean, terrain shadow 92/92); 141 (regress voidbool, PASS, lists as run 138); 140 (regress abitidy2, PASS, lists as run 138); 139 (regress masterlist, PASS, lists as run 121: it ran at 05:51 before runs 133-138 and is recorded after them); 138 (regress rl5, regresslists5, PASS, the first lists with `terrain_shadow`; baseline `build-runs/regress-rl5.out`); 137 (regress cores15, PASS, lists as run 132); 136 (regress glue16, PASS, lists as run 132); 135 (regress glue13, PASS, lists as run 132); 134 (regress cores14c, PASS, lists as run 132); 133 (regress cores12b, PASS, lists as run 132); 132 (regress rl4, regresslists4, PASS, the first lists with `lotdev_shadow`; recorded after masterlist's regress had finished, whose record follows); 131 (regress seeding, PASS, lists as run 121); 128 (regress apply11traffic, PASS, lists as run 121); 127 (regress autoloadfix, PASS, lists as run 121); 126 (regress apply13, PASS, lists as run 121); 125 (regress apply12, PASS, lists as run 121); 124 (regress cores16, PASS); 123 (regress cores12a, FAIL: no city load; merge reverted); 122 (regress abitidy, PASS, lists as run 118); 121 (regress rl3, PASS) is the first with the traffic lists. See `docs/manual-checks.md`.
- **Latest in-game run:** 147 (disasters shadow exact - fire 256/256, riot 352/352, no differences); 145 (disasters shadow, riot 267/267 exact, fire 303/343 (DS38 + a deferred-spread lot lag)); 144 (disasters shadow after the riot walk fix; riot 396/396 exact, fire 212/215); 143 (disasters shadow after the resync fix, clean (one ref warning)); 142 (disasters shadow before the resync and riot-walk fixes, clean); 141 (regress voidbool, PASS, lists as run 138); 140 (regress abitidy2, PASS, lists as run 138); 139 (regress masterlist, PASS, lists as run 121: it ran at 05:51 before runs 133-138 and is recorded after them); 138 (regress rl5, regresslists5, PASS, the first lists with `terrain_shadow`; baseline `build-runs/regress-rl5.out`); 137 (regress cores15, PASS, lists as run 132); 136 (regress glue16, PASS, lists as run 132); 135 (regress glue13, PASS, lists as run 132); 134 (regress cores14c, PASS, lists as run 132); 133 (regress cores12b, PASS, lists as run 132); 132 (regress rl4, regresslists4, PASS, the first lists with `lotdev_shadow`; recorded after masterlist's regress had finished, whose record follows); 131 (regress seeding, PASS, lists as run 121); 128 (regress apply11traffic, PASS, lists as run 121); 127 (regress autoloadfix, PASS, lists as run 121); 126 (regress apply13, PASS, lists as run 121); 125 (regress apply12, PASS, lists as run 121); 124 (regress cores16, PASS); 123 (regress cores12a, FAIL: no city load; merge reverted); 122 (regress abitidy, PASS, lists as run 118); 121 (regress rl3, PASS) is the first with the traffic lists. See `docs/manual-checks.md`.

## Lanes and open items as of 2026-10-14/15

### Lanes

Every lane is running (resumed 2026-10-14 14:05 after an API session limit stopped them all at 13:57; resume with
`SendMessage`, and tell the lane which of its queued runs finished meanwhile). **Engine lanes test their own DLL
in game** through the run queue (`docs/harness.md`; the queue is `tools/game_queue.ps1 -Slots 6`, started detached
from the main checkout, log `build-runs/game-queue.log`) and must report request labels and verdict lines. Lane
prompts are in `build-runs/lane-prompts/` (gitignored, on disk); new engine lanes start from
`engine-lane-header.txt` + `selftest-addendum.txt` + the task. A worktree lane cannot read files outside its
worktree: paste what it needs into the message, or copy files into its `build-runs/`. Resume a stopped lane with
`SendMessage` to its agent ID.

The lane roster is the generated table under "Wind-down 2026-10-14" above: one row per lane, from
`docs/lanes/<lane>.md`. The older `| Lane | Agent ID | Worktree | Reserved | State |` roster was merged
into it (a lane's ABI/seam reservation is in its row's "State at stop" cell); regenerate with
`python tools/gen_handoff_tables.py`.

Merged today, tooling: harness switches (`6147be6`, `0786064`), modal-dialog spec (`baa5d0a`), game slots and the
run queue (`9e0fa74`: up to 3 games at once; `regress -Parallel` 164 s, `-Turbo -Parallel` 94 s to 2032), the
dialog closer (`dba09eb`: `close_dialogs = 1`; in game it closed 5 advisor popups by button with popups on).
The traffic simulator answers TS35-TS49 (re-answers8) go to the traffic glue lane after its phase 1.

### Open items, most useful first

1. **Development glue works in game** (devglue2, merged `38524aa`): catalogue 744/744 types and 281 families, the
   shadow exact, the replacement grows like the original at equal pacing. Open: our development unit is about 3x
   slower per game day; DG28-DG31 (DG31: the original's results depend on game days per real second).
2. **Landfill game tiles** (LG27-LG29): every tile fails at the last step (run 103). Answered: the original calls
   `SetDefaultExemplar` on the holder sub-object at +0x20 (layouts.md); apply9b applies it.
3. **Zones:** the shadow differs only by ZG17 (answered; apply9b applies it). The dezone tool path is untested in
   game. Phase 2 (zone developer, lot manager) and the lot developer glue have not started.
4. **Traffic glue** phase 1 merged (`850b567`, `enable = traffic`, clean in game); phase 2 running (TS35-TS49, the
   O(N^2) lot scan, host callbacks, a traffic shadow for the M5 exit). **Tract glue** lane running.
5. Residential buzzards never trigger in Konradshohe; a growth probe on a large city is optional (attempt cost grows
   with lots).
6. **QUESTIONS.md restructure** (open index, archive answered sections): after re-answers8 merges. The audit proposal
   is in this session's notes; the short version is in §6.
7. The x87 precision trigger (what raises the main thread to 53-bit during app init) is still unknown; the 24-bit
   re-assert makes it harmless (foundations §18-§19). `setup_ghidra.ps1 -Mac` is untested.

## Superseded HANDOFF text (replaced on 2026-10-20)

The HANDOFF's opening paragraph and its §1, §4, §6 and §7 as they were before the restructure. §6's lock and
merge-order procedure is obsolete since the merge train ([docs/merge-train.md](../merge-train.md)); its current
form is [docs/ops/coordinating.md](../ops/coordinating.md).

# Handoff (start here)

This file is for the agents that take turns on this repo, the coordinator first. Read it before you start and update
§0 before you stop. Milestones are tracked in the [Progress checklist](../PROGRESS.md); the in-game run
log is [docs/manual-checks.md](../manual-checks.md); history is `git log`. This file holds only the current state and the
operating knowledge that is still true.

## 1. The project

We are reverse engineering SimCity 4 Deluxe (GOG 1.1.641, exe SHA-256 `3bc5c7fe…a4b`) and building a clean-room,
GPL-3.0 reimplementation that replaces the game piece by piece, OpenRCT2-style: a Rust core (`engine/core`), file
formats (`engine/formats`), a C ABI (`engine/capi`), a C++ GZCOM plugin (`engine/plugin`, builds `FreeCity4.dll`,
loaded from the user's `Plugins` folder), JSON conformance vectors (`engine/conformance`) and C++ tests
(`engine/tests`). Specs for implementers are in `docs/specs/`, interface/ABI facts in `docs/abi/`, the layout and
roadmap in `docs/ARCHITECTURE.md`, build and ini keys in `engine/README.md`.

## 4. Milestones

See the [Progress checklist](../PROGRESS.md). Keep it current at every merge.

## 6. Coordinating lanes

- **Spawn** one clean-room engine agent per task, in an isolated worktree, from a prompt like those in
  `build-runs/lane-prompts/` (clean-room rules, first steps, build/test commands, shared-file rules, the task, the
  report). Reserve an ABI number and seam op per lane. A new worktree may start from an old commit: the lane runs
  `git merge main` first.
- **Gate** before every merge: `build-runs/coord-full-verify.ps1 -Engine <worktree>\engine -Tag <tag>` (cargo test
  summed over every `test result:` line, clippy `-D warnings`, CMake build + ctest; a private `_MSPDBSRV_ENDPOINT_`
  per gate). Then `rustfmt --check --edition 2021` on the touched `.rs` files from `engine/`. Flaky under load: two
  flora lock-timing tests; rerun before blaming the lane.
- **Merge order:** merge one lane, then have the next lane `git merge main` itself, or, if
  `git merge-tree --write-tree --name-only main <branch>` prints only a hash, merge main into its worktree yourself
  (`git -C <wt> -c submodule.recurse=false ...`, add files explicitly: `commit -a` fails on the worktree's submodule
  pointer). Renumber seam ops / switch counts at that point. Bump the ABI version once for the whole batch with
  `python tools/bump_abi.py --batch <batch-id> --lanes laneA,laneB` (never per lane, and never by hand).
  `abi_entries.inc` is generated and no longer in git: do not merge, renumber or revert it - the build regenerates
  it from `gen_abi_entries.py` before compiling. Merge with `git merge --no-ff -F <msgfile>`.
- **After the merge:** copy the DLL/PDB to `build-runs/dlls/FreeCity4-<tag>.*`, run regress, record the run in
  `docs/manual-checks.md`, file the lane's questions in `docs/specs/QUESTIONS.md`, tick docs/PROGRESS.md.
- **Pause / stop:** `TaskStop`, resume with `SendMessage`. After a hard stop, check the worktree's *unstaged* diff:
  the agent may have been writing the fix to something it had just committed.
- **QUESTIONS.md:** answered entries are struck and kept (`~~**ID**~~ ... ANSWERED`). Open entries are found with
  `grep -nE '^(- )?\*\*[A-Z]{2}[0-9]+' docs/specs/QUESTIONS.md`. Planned: move fully answered sections to
  `QUESTIONS-answered.md`, add a generated open-question index and a prefix legend (first-generation PR/PW numbers
  were reused); new questions continue the subsystem's existing prefix.

## 7. Waiting on the user

1. Two human checks: the budget window's look and the cheats (`docs/manual-checks.md`).
2. Upstream the SDK errata (public; `docs/sdk-errata-draft.md` needs a re-run first).
3. Publish the repo (public).
4. Delete the local KillaVolt clone (`external/sc4-decomp`, `external/extracted`); download the Mac Rev A binary.
5. Keep or delete `build-runs/windown-backup-20261013/` (every lane it covered has committed).

# Next steps

The one pick-up document for the next coordinator session. It states what we do next and nothing else. Every write
replaces the whole file: no history, no logs, no "previously". Evidence lives in git, in `docs/lanes/` and in
`build-runs/`, never here. Where another document disagrees with this one, this one wins.

State as of 2026-10-27: **run 7 is halted.** No lanes, trains, operator or conductor are running; census-1027 has
finished (the shared game queue service is still up and idle). The user approved the plan below. Step 1 is next and
is run by the coordinator directly.

## 1. The goal, worked backwards

**Goal:** a standalone game written in Rust/C++ that behaves exactly like SimCity 4. Replacing `SimCity 4.exe` is not
the goal by itself; replacing it with a game that is exactly like it is.

**Exactly like (agreed with the user):** the same inputs under the same work schedule give identical results.
SimCity 4 itself is not deterministic. Its results depend on machine speed through CPU-cycle time limits, on wall-clock
timers (the 24-hour clock, the animation tick that drives the lot and prop developers, terrain, fires and automata,
fire spread every 1000 ms), on clock-derived random seeds, on one C runtime `rand()` sequence shared with audio and
automata, and on heap addresses written into saves as object IDs (`docs/specs/simulator.md` §5.5 and §5.7,
`foundations.md` §3.4, §6, §12 and §18, `clock24.md`, `INVENTORY-M6.md`). So both games run one **fixed schedule**
(section 2), and the original run under that schedule is the reference.

For this to be true at the end:
1. Every behaviour of the exe exists in our code: simulation (35% of the exe's game code), renderer (22%), UI (20%),
   framework and data (11%), audio (4%), region (2%).
2. Our own host runs it, with no exe.
3. Each behaviour is proven exact at a boundary that still exists once the exe is gone: the **save file** (city
   state, compared as decompressed records), the **draw calls** into the graphics driver interface `cIGZGDriver`,
   **sound events**, **UI window state** and **resource lookups**. The C ABI seam `fc4.h` is scaffolding between our
   core and the original's objects; it disappears in the standalone game, so checks at that seam are debugging aids,
   not proof.

## 2. The fixed schedule

The schedule is written once, in `docs/specs/simulator.md` §5.7 (with the seed, `rand()`, FPU and object-ID rules
in `foundations.md`), and both games follow it. Everything on the original's side is done by `FreeCity4.dll` at run
time (in-memory hooks and writes; never an edit of the exe file).

| Part | Rule | Our game | The original |
|---|---|---|---|
| Time | one virtual clock; every wall-time source reads it; a fixed number of frames per game day | `sim_fixed_step` already virtualises the scheduler's clock (`cFC4Simulator.cpp`) | hook `QueryPerformanceCounter`, `timeGetTime`, `GetTickCount` and the local-time source behind `cRZTime` so all read the virtual clock; today only our scheduler's clock is virtual. The sliced-call timer is the next row |
| Calendar | a day advances only after every day-level agent has finished (simulator.md §5.2) | `fc4_core` simulator | our simulator replacement `cFC4Simulator` already drives every agent |
| Idle work | at each day's settle point every idle agent runs in one fixed order written in the spec (the building developer waits for its 12 developers, so order changes results by design); each agent gets a fixed number of deliveries; what runs while the game is paused is written down (the original runs idle work while paused) | one `SimIdle` per agent runs the whole cycle (`FC4Pacing.h`, deterministic pacing, the default without shadows) | original idle agents always answer "done", so the core's re-delivery never repeats them: give each a counted number of deliveries. Known per-delivery workers: prop developer (20, already paced), neighbours (neighbors.md), plumbing restart (flammability.md); find the rest. The caps of 1024 deliveries and the 22 ms `data3` (simulator.rs) must never cut |
| Time limits | large but finite: no sliced call that does work is ever cut; the building developer's idle spin (building-development.md §4.4: it re-sums demand until the 3 ms timer expires) still ends | budgets unlimited (building-development §4.4.1 default 0; the spin has no effect) | the sliced-call timer is `RDTSC + ms × MillisecsToCycles` (small-owners-b9.md). Either scale `MillisecsToCycles` (`0x00B0A83C`, simulator.md §5.5) by a measured factor F, keeping ms × M below about 2^62 (a larger product converts to an already-expired deadline, foundations §18.1), and set `CyclesToMillisecs` (`0x00B0A840`) to match; or hook the timer so it reads a counter that advances per read (cut points then depend on work done, not on the machine). Step 1 picks one; F and 2F (or step and step/2) must give identical results |
| Posted messages | delivered in posting order at the day's settle point, before the next day's agents (§5.7 item 4) | `PacingDefer` / `PacingFlush` | `PacingFlush` already drains the message server at the settle point. Still open: posts from outside the simulator (24-hour clock hour messages, animation tick, UI), the 64-tick / queue-not-shrinking exit and the 10 s walk cap (simulator.rs) |
| Random numbers | every generator's seed is derived from the session seed, derive(S, K), per the generator table of foundations §6.3; member and temporary generators get the seeds §6.4 prescribes; the C runtime `rand()` sequence is one sequence shared by the original's code and ours | seeded generators (`rng.rs`, `seeding.rs`); add a host `rand` callback so cores running inside the game draw from the game's own `rand()` state (today each core keeps a private copy starting at state 1, foundations §12.6) | the file-local generators are seeded during static initialisation from `cRZTime()`, before the plugin loads, so re-seed each one at DLL load with the game's own `Seed` and derive(S, K); force member and temporary seeds; audio, radio, music and automata off in test runs, or log the `rand()` draw count at every delivery |
| Object IDs | objects are numbered 1, 2, 3, … in registration order (foundations §3.4 *[choice]*) | already | hook the original's ID assignment at registration (`0x005D43F0`) so it numbers objects the same way |
| FPU precision | 24-bit on the simulation thread at every agent delivery (foundations §18); paths reached from Lua run at 53-bit (§18.4), so write down what applies there | not applicable (Rust floats are explicit) | pin it at each delivery if measurements show drift |
| Frames | nothing above depends on frames or wall time; rendering can run at any frame rate | | |

Comparison: saves at fixed checkpoint days, compared **record by record after decompression** (`golden_dbpf` already
ignores container layout; our QFS compressor is our own design, so compressed bytes never match). Rendered records
such as the thumbnail are compared by the renderer checks (step 6), not here.

## 3. The metric (agreed with the user)

**Exact-replaced:** the share of the exe's game code (bytes, `tools/coverage.py` attribution) whose owners are
switched off in corpus runs (our code runs instead) while every checkpoint of every scenario that reaches them is
identical to the reference. **Reach** is the ceiling: the share of game code the corpus executes at all in the
original. Specified and Implemented stay as the pipeline in front. `tools/coverage.py` reports exactly these four
(definitions in `docs/COVERAGE.md`); until step 2 exists, Exact-replaced reads "not measured".

Deleted from the tools and docs on 2026-10-27, never to be quoted again: the house-city "Verified", "strict",
single-method credit, shadow-based credit and "Replaced" measured on the house run. Shadows (`<x>_shadow = 1`) stay
only as a debugging aid.

Today (`docs/coverage-report.md`, Reach from the 177 clean runs of census-1027): Specified 91.9%, Implemented 63.0%,
Exact-replaced not measured, Reach 42.4% of game code. The corpus reaches 45.0% of the implemented owners' code, so
Exact-replaced cannot exceed 28.3% of game code until the corpus reaches more.

## 4. The loops

| Loop | What runs | Duration |
|---|---|---|
| **Reference recording** | the original under the fixed schedule, once per scenario; checkpoint saves into an immutable store (golden27's `tools/golden.py` store) | per run about 15 s outside the simulation (plain runs, game-queue.log) plus the window at the schedule's speed, which step 1 measures (golden27's budget-only candidates took 44.5 s and 142 s for their windows). 189 scenarios on 12 slots (one queue with `-SlotList`, as census-1027 ran): estimate 30-90 min. Redone only when the corpus or the schedule changes |
| **Per change (inner loop)** | incremental build, then our game in replace mode on the scenarios whose reach includes the changed owners, compared with the references | estimate 3-8 min: build 1-2 min, about 20 scenarios at 45-140 s each on 12 slots. The run is reproducible, so the lane's own result is the credit |
| **Landing** | merge, gate (409 s today), the same check on main | estimate under 15 min |
| **Nightly** | the whole corpus, ours against references | the same as one reference recording |
| **Later: standalone headless** | our own exe, no game process, against the same references | seconds per scenario (estimate) |

Today, for comparison (run 7, measured): shadow runs go at 0.39 game days/s against about 3 for plain runs; a census
costs 22 slot-hours (about 5 h on 6 slots); credit arrived only after a merge-train regress on main; a lane took a
median of about 3 h from first commit to landing; game slots were about a third busy.

## 5. Steps

### Step 1: hold the original to the fixed schedule (coordinator; first report at 6 h, stop and report at 2 days)

1. Write the schedule of section 2 into simulator.md §5.7 and foundations.md: the time rule, the idle-agent order,
   per-agent delivery counts and the paused rule, the time-limit choice, message delivery, derive(S, K) seeds, the shared `rand()` and
   object IDs.
2. Implement it for the original in `cFC4Simulator` / `FC4Pacing` behind one switch (`pacing = fixed` or similar):
   virtual clock hooks, counted idle deliveries, the time-limit choice, external posts drained, static generators
   re-seeded and member seeds forced, ID numbering, audio, radio, music and automata off (in-memory writes by the DLL
   are allowed; exe file edits are not).
3. Make checkpoint saves: the golden store keeps only `end.sc4` today, and a full save took 10.8 s on Konradshohe
   (60 s watchdog grace), so save at day 30 and day 60 inside one run.
4. Measure with golden27's cities (FC4 Test/Konradshohe, Maxisland/West Weburg, Paradis/Mitras), every subsystem
   original except the scheduler:
   - **Pass:** 3 runs of each city give identical decompressed records at day 30 and day 60.
   - F and 2F give identical records; N and 2N deliveries give identical records.
   - Speed: game days/s under the schedule, including the building developer's spin (3 ms × F per day).
   - Sanity (not a gate): population and funds stay within golden27's real-time envelopes, so the schedule is still
     the game.
5. Finding what still varies: diff records per checkpoint; then per-delivery digests (new work: `FC4Observer`'s
   DIGEST lines are monthly and cover funds, R/C/I, demand, income and expense only), a hash of the records an agent
   owns plus the `rand()` draw count after each delivery, to find the first delivery that differs. Suspects not yet
   ruled out: worker threads (model background loader, sound mixer; INVENTORY-M6 §8), heap-address-dependent
   ordering other than IDs, FPU precision.
6. **If after 2 days unexplained variation remains:** report the numbers to the user. Fallback: golden27's envelopes
   for the simulation, and paused-save checks (step 6) for everything else.

### Step 2: references and the check command

1. Extend the golden store: the schedule is part of the key; scheduled keys require exact equality (no envelopes).
   Today the key hashes the observer DLL and the harness sources (golden.py), so every DLL rebuild or harness edit
   would invalidate every reference: key on the schedule code and a pinned reference DLL instead, and refuse a
   candidate whose schedule code differs.
2. Make replace mode able to match: cores draw from the game's `rand()` through the host callback, seed with the same
   derive(S, K), and register objects in the original's order.
3. Record references for the census scenarios.
4. `check`: owners to switch on (replace mode) and scenarios chosen by reach (`tools/suite_select.py`,
   `tools/suite_coverage.py --select`); compares checkpoint records and prints the first differing day, record and
   owner. On a difference it switches owners back to the original one at a time (or by halves) to find the culprit.

### Step 3: Exact-replaced in `tools/coverage.py`, and the landing rule

The old credit machinery is gone (the coverage tool reports the four numbers of section 3; `creditcheck.py` and the
merge train's credit hold were deleted). Step 3 adds Exact-replaced from `check` results and makes an exact `check`
the landing rule for an engine lane that switches owners on.

### Step 4: restart engine lanes (about 9, no operator layer)

Each lane takes implemented owners, switches them on in replace mode and makes `check` exact on their scenarios. Each
names the owners and the scenarios it will make exact. Specs and RE answers feed them as before (clean room unchanged).

### Step 5: the corpus, built from reach (1 lane, ongoing)

- Exact-replaced cannot exceed what the corpus reaches (section 3).
- In census-1027, 8,080 of 30,174 actions failed (26.8%: insufficient funds, blocked placement). Fix the action
  drivers so scripts do what they intend.
- 10 of 189 scenarios wrote no reach counters; find out why.
- Add scenarios for unreached code (tools, panels, disasters, region, save and load, options) and UI input scripts.
  The user records short play sessions for what scripts cannot reach.

### Step 6: the standalone track (1-2 lanes, in parallel from step 2)

1. **Save round trip.** What exists: a DBPF writer (`PackageBuilder`, `engine/formats` dbpf.rs) and class-store save
   and load (store.rs). A round trip needs all 128 saved classes plus the terrain and view records ("city and save
   file" shows 78% implemented, but that is one 35 KB row). Milestone: on saves written by 1.1.641, our load-and-write
   gives identical records after decompression and ID renumbering; the shipped saves (404 of 417 written by older
   builds) only need to come back unchanged on a second write. Needs no game runs.
2. **Headless host.** Our exe loads the player's data files and a save, runs N days under the fixed schedule, writes a
   save, compared with the same references. It implements the host side of `fc4.h` (58 host tables, 2,366
   callbacks) over our own world model; there is no host crate yet. First the host and city specs INVENTORY-M6 §9
   asks for; note the zone developer's `Init` needs the 3D view (§8).
3. **Renderer and UI checks:** capture the `cIGZGDriver` call stream and the UI window state on paused saves in the
   original and in ours (deterministic even without step 1); then live, under the fixed schedule.
4. Then a standalone renderer behind the same interface, UI, audio.

## 6. What we stopped, and why (do not restart)

- **Record/replay at the `fc4.h` seam as the main loop** (former E1): it checks a seam the standalone game deletes;
  traces depend on the exact order of our core's callbacks, so they go stale whenever an
  owner's queries change; recording overhead grew from 14.3x to 24.3x (trace-replay31, Aquaton 186.6 s against
  7.7 s) with compression already on a background thread; one 60-day window with 5 owners is 2.2 GB raw. The 5
  covered owners (budget, power, pollution, traffic, terrain) keep replay as unit tests. The census replay tools
  (`suite_replay.py`, `replay_coverage.py`) were deleted.
- **House-city verified, strict and shadow-based credit**, the census with shadows as the source of credit, and the
  tools that computed them (`coverage_methods.py`, `compare_metrics.py`, `creditcheck.py`, `ledger.py`,
  `suite_reach_gaps.py`, `coverage-shadows.tsv`).
- **Single-method credit lanes** (`method-*`): all method evidence together was 23,555 bytes, 0.4% of game code.
- **Merge-train regress as the credit gate:** it stays only as a landing check until step 3 adds `check`.

## 7. What the user does (5-minute checks)

Any check that costs the coordinator about an hour once and the user about 5 minutes: comparing UI screens, clicking
through dialogs, and playing and recording short sessions for the corpus. Give exact steps and what to look for.

## 8. Picking up from the halt

**Census-1027** (finished: 189 scenarios, 177 clean, 179 with reach counters; `build-runs/suite/census-1027/`). It
gives Reach (section 3); `build-runs/suite/CURRENT` names it, so `python tools/coverage.py` uses it by default.

**Saved work** (all local; nothing was pushed or deleted):
- 51 branches `halt-1027/<worktree>` hold every lane worktree's work.
- Held game requests: `build-runs/game-queue-halted-1027/`. Lane specs: `build-runs/run7/specs/`.

**Merge queue** (50 entries, `python tools/merge-train.py status`). The credit hold is gone, so many entries now show
READY: none of that is a judgement. Train 296 (re-hg32, method-www31, trace-replay31) was killed mid-regress. Re-judge
every entry against this plan and land only explicit batches (`merge-batch --lanes`):
- Drop the `method-*` entries.
- `trace-replay31`: land only its load-stall fix if its gate passes; no further recorder work.
- Tools that speed the loop (`build-speed32`) and spec answers (`re-*`) can land after a gate.
- `replace-*`, `shadow-*`, `glue-*` and the other engine entries wait for `check` (step 2) and land by its result.
- Entries that touch the deleted tools (coverage credit, ledger, census replay) need rebasing or dropping.

**Known state.**
- `effectsres` is off the replace list after a start-up crash (EF28); spec question re-ef32 is answered.
- `git push` reports that the repository moved to https://github.com/tloriato/OpenSC4.git; pushes still work.

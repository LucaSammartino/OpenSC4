# Next steps

The one pick-up document for the next coordinator session. It states what we do next and nothing else. Every write
replaces the whole file: no history, no logs, no "previously". Evidence lives in git, in `docs/lanes/` and in
`build-runs/`, never here. Where another document disagrees with this one, this one wins.

State as of 2026-10-27: **run 7 is halted.** No lanes, trains, operator or conductor are running (the shared game
queue service is still up and idle). The user approved the plan below. **Step 1 is done** (section 5); step 2 is next
and is run by the coordinator directly.

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

The schedule is `docs/specs/simulator.md` §5.8: clock, timer callbacks, time limits, wall clock, random numbers,
object IDs and what only the original needs (heap, pools, DLL base, audio and loader off). The original follows it
through `FreeCity4.dll` (`engine/plugin/FC4Schedule.cpp`, `schedule = fixed` in FreeCity4.ini, `golden.py --schedule
fixed`; in-memory hooks only, never an edit of the exe file).

What our game still needs to follow it (step 2):
- `rand()`: cores running inside the game draw from the game's own `rand()` state through a host callback (today each
  core keeps a private copy starting at state 1, foundations §12.6).
- Seeds: derive(S, K) for every generator, as the original's re-seeding does (foundations §6.3, §6.4).
- Object IDs: 1, 2, 3, ... per save, as the original's writer now numbers them.
- Address order: results depend on heap address order wherever a container is keyed by pointer (the traffic network
  map's buckets are address mod 193; agent order, foundations §13). The original's order is now fixed by its heap
  (lowest address first from 0x20000000). Our objects must sit in the same order: find every pointer-keyed container
  a replaced owner touches and either allocate in the original's order or make the spec state the order it implies.

Comparison: saves at day 30 and day 60, compared **record by record after decompression** (`golden_dbpf` ignores
container layout; our QFS compressor is our own design, so compressed bytes never match). Rendered records such as
the thumbnail are compared by the renderer checks (step 6), not here.

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
| **Reference recording** | the original under the fixed schedule, once per scenario; day-30 and day-60 saves into an immutable store (`tools/golden.py`) | measured, 6 games in parallel: 1.6-7.3 game days/s (Konradshohe 4.4-7.3, West Weburg 2.5-3.8, Mitras 1.6-2.4), plus about 15 s outside the simulation and about 1.2 s a save. 189 scenarios on 12 slots: estimate 15-40 min. Redone only when the corpus or the schedule changes |
| **Per change (inner loop)** | incremental build, then our game in replace mode on the scenarios whose reach includes the changed owners, compared with the references | estimate 3-8 min: build 1-2 min, about 20 scenarios at 25-55 s each on 12 slots. The run is reproducible, so the lane's own result is the credit |
| **Landing** | merge, gate (409 s today), the same check on main | estimate under 15 min |
| **Nightly** | the whole corpus, ours against references | the same as one reference recording |
| **Later: standalone headless** | our own exe, no game process, against the same references | seconds per scenario (estimate) |

## 5. Steps

### Step 1: the original under the fixed schedule (done 2026-10-27, commit 8d3c3168d)

Konradshohe, Mitras and West Weburg, every subsystem original except the scheduler, 6 games in parallel:
- 3 runs of each city: identical decompressed records at day 30 and day 60 (152, 159 and 159/158 records).
- N and 2N reads before a time limit expires: identical records in all 6 windows.
- Speed: in section 4.
- Sanity (not a gate): population and funds sit 0.2-1.5% outside golden27's real-time envelopes (Konradshohe day 60:
  industrial 7907 against 7770-7791, funds 1033 against 1039-1051). The likely cause is fewer timer and animation
  callbacks per game day at about one frame a day. It is the schedule's game, not a defect; the references are the
  schedule's results.

The references for these 6 windows are in `build-runs/golden`. Their keys include the hash of the DLL those runs
used, so any rebuild gives new keys until step 2.1 changes the key.

### Step 2: references and the check command

1. Extend the golden store: the schedule is part of the key; scheduled keys require exact equality (no envelopes).
   Today the key hashes the observer DLL and the harness sources (golden.py), so every DLL rebuild or harness edit
   would invalidate every reference: key on the schedule code and a pinned reference DLL instead, and refuse a
   candidate whose schedule code differs.
2. Make replace mode able to match (section 2): the host `rand()` callback, derive(S, K) seeds, object IDs and
   address order.
3. Record references for the census scenarios.
4. `check`: owners to switch on (replace mode) and scenarios chosen by reach (`tools/suite_select.py`,
   `tools/suite_coverage.py --select`); compares checkpoint records and prints the first differing day, record and
   owner. On a difference it switches owners back to the original one at a time (or by halves) to find the culprit.
   The schedule's per-delivery digest lines (`rand()` state, generator hash, target class) find the first delivery
   that differs.

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

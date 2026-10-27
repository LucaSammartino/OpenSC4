# Next steps

The one pick-up document for the next coordinator session. It states what we do next and nothing else. Every write
replaces the whole file: no history, no logs, no "previously". Work that is done and verified is deleted from it.
Evidence lives in git, in `docs/lanes/` and in `build-runs/`, never here. Where another document disagrees with this
one, this one wins.

State as of 2026-10-27: **run 7 is halted.** No lanes, trains, operator or conductor are running (the shared game
queue service is up). The user approved the plan below. **Step 2 is in progress**, run by the coordinator directly.

## 1. The goal, worked backwards

**Goal:** a standalone game written in Rust/C++ that behaves exactly like SimCity 4. Replacing `SimCity 4.exe` is not
the goal by itself; replacing it with a game that is exactly like it is.

**Exactly like (agreed with the user):** the same inputs under the same work schedule give identical results.
SimCity 4 itself is not deterministic (machine speed through CPU-cycle time limits, wall-clock timers, clock-derived
seeds, one shared C runtime `rand()`, heap addresses as object IDs). So both games run one **fixed schedule**
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

The schedule is `docs/specs/simulator.md` §5.8. The original follows it through `FreeCity4.dll`
(`engine/plugin/FC4Schedule.cpp`, `schedule = fixed`; in-memory hooks only, never an edit of the exe file).

**The reference store** (`tools/golden.py`, `build-runs/golden`) keys a fixed-schedule reference on the schedule code
(sha256 of `FC4Schedule.cpp` and `.h`), the pinned reference DLL and the run settings; it requires exact equality and
refuses a candidate whose schedule code differs. Any edit of `FC4Schedule.cpp` or `.h` means: rebuild, `python
tools/golden.py pin --dll <new dll> --repin`, re-record. Pinned now: sha 3c2afcf8, schedule code 1107b55f0944ecb8
(commit 562a1f41e); its 55 references are recorded (walk census on) and the original repeats
every one of them (55 of 55 exact). Fixed-schedule runs end at the end date's save (`autotest_end = terminate`): the original's quit after it
can crash in the city teardown.

**Comparison:** the save at each scenario's end date, record by record after decompression (`golden_dbpf` ignores
container layout; our QFS compressor is our own design), plus the DIGEST lines and the schedule's per-delivery digests.

**What replace mode still lacks to match** (step 2.2; the host `rand()`, derive(S, K) seeds and object IDs are in):
- Address order: where the original walks a container keyed by object address, the schedule makes it walk in creation
  order (or the order its spec states); simulator.md §5.8's Address order row lists every such walk, and each is in
  its owner's spec. Two detectors run the original with other addresses, so a result that still follows addresses
  changes: `GOLDEN_INI_EXTRA="heap perturb = holes"` (or `= reverse`) `python tools/check.py run --dll <copy of the
  build> --replace none`. Measured on the pin: holes 54 of 55 exact, reverse 53 of 55, plain 55 of 55. Open: the
  watercraft record `0x096E75F5` and the watercraft manager `0xA98F9D28` in paradis-jinae (holes, reverse) and
  maxisland-east-weburg (reverse). `walk census = on` digests the order of every hash-table walk
  (`hashtable::begin`, 0x004B0390) per call site; red-black tree walks are inline (299 successor loops in the exe) and
  are not counted, so find those from the record that differs.
- Two clock seeds still keep the schedule's `time()`/`clock()` instead of derive(S, K): the flammability generator's
  zero-state reseed (`cRZRandom::RandomDoubleUniform`, 0x0091373B) and the growth decline seed. Map them only if
  `check` shows them.

## 3. The metric (agreed with the user)

**Exact-replaced:** the share of the exe's game code (bytes, `tools/coverage.py` attribution) whose owners are
switched off in corpus runs (our code runs instead) while every checkpoint of every scenario that reaches them is
identical to the reference. **Reach** is the ceiling: the share of game code the corpus executes at all in the
original. Specified and Implemented stay as the pipeline in front. `tools/coverage.py` reports exactly these four
(definitions in `docs/COVERAGE.md`); until step 3, Exact-replaced reads "not measured".

Deleted on 2026-10-27, never to be quoted again: the house-city "Verified", "strict", single-method credit,
shadow-based credit and "Replaced" measured on the house run. Shadows (`<x>_shadow = 1`) stay only as a debugging aid.

Today (`docs/coverage-report.md`, Reach from the 177 clean runs of census-1027): Specified 91.9%, Implemented 63.0%,
Exact-replaced not measured, Reach 42.4% of game code. The corpus reaches 45.0% of the implemented owners' code, so
Exact-replaced cannot exceed 28.3% of game code until the corpus reaches more.

## 4. The loops

| Loop | What runs | Duration |
|---|---|---|
| **Reference recording** | the original under the fixed schedule, once per scenario (`python tools/check.py record`) | measured: 1.6-7.3 game days/s per game, about 15 s outside the simulation, about 1.2 s a save; 12 games in parallel. Redone only when the corpus, the schedule or the run settings change |
| **Per change (inner loop)** | incremental build, then our game in replace mode on the owner's cover scenarios against the references (`python tools/check.py run`) | estimate 3-8 min |
| **Landing** | merge, gate (409 s today), the same check on main | estimate under 15 min |
| **Nightly** | the whole corpus, ours against references | the same as one reference recording |
| **Later: standalone headless** | our own exe, no game process, against the same references | seconds per scenario (estimate) |

## 5. Steps

### Step 2: references and the check command (in progress)

The scenarios are the owner cover `build-runs/golden/owner-scenarios.tsv` (`python tools/coverage.py --owner-scenarios
<path>`): per replace key, a greedy cover of its census reach, 55 scenarios. Lua scenarios and the two clockless
Berlin cities (set `clockless` in `engine/tests/scenarios/suite.tsv`) are not run.

1. **Each owner alone** over its cover: `python tools/check.py run --dll <copy of the build> --replace each` (an
   owner that needs others runs with them: mysims with advisors, netrules with netmgr and nettool). It writes
   `build-runs/golden/owner-check.tsv`: per run, exact, or the first differing delivery, DIGEST date and record
   (with its owner). Measured once, before most address-order walks: 13 of 108 runs exact (aura 2, gzfw 2,
   protection 2, budget, citydetail, industrial, nettool, span565, tract, tutorial), 72 outside, 23 errors (a
   foreign exception or a stall: automata 9, disasters 3, netmgr 3, regionalcity 3, effects, netrules, span8888,
   terrain, traffic). The errors are engine findings for step 4. Run it again on the current pin next.
2. **Address order** (section 2): the two watercraft scenarios, then whatever the new table shows. The two clock
   seeds only if `check` shows them.

### Step 3: Exact-replaced in `tools/coverage.py`, and the landing rule

Step 3 adds Exact-replaced from `check` results and makes an exact `check` the landing rule for an engine lane that
switches owners on.

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
   original and in ours (deterministic even without the schedule); then live, under the fixed schedule.
4. Then a standalone renderer behind the same interface, UI, audio.

## 6. What we stopped, and why (do not restart)

- **Record/replay at the `fc4.h` seam as the main loop** (former E1): it checks a seam the standalone game deletes;
  traces depend on the exact order of our core's callbacks, so they go stale whenever an owner's queries change;
  recording overhead grew from 14.3x to 24.3x (trace-replay31, Aquaton 186.6 s against 7.7 s); one 60-day window with
  5 owners is 2.2 GB raw. The 5 covered owners (budget, power, pollution, traffic, terrain) keep replay as unit tests.
- **House-city verified, strict and shadow-based credit**, the census with shadows as the source of credit, and the
  tools that computed them.
- **Single-method credit lanes** (`method-*`): all method evidence together was 23,555 bytes, 0.4% of game code.
- **Merge-train regress as the credit gate:** it stays only as a landing check until step 3 adds `check`.

## 7. What the user does (5-minute checks)

Any check that costs the coordinator about an hour once and the user about 5 minutes: comparing UI screens, clicking
through dialogs, and playing and recording short sessions for the corpus. Give exact steps and what to look for.

## 8. Picking up from the halt

**Census-1027** (189 scenarios, 177 clean, 179 with reach counters; `build-runs/suite/census-1027/`) gives Reach;
`build-runs/suite/CURRENT` names it, so `python tools/coverage.py` uses it by default.

**Saved work** (all local; nothing was pushed or deleted):
- 51 branches `halt-1027/<worktree>` hold every lane worktree's work.
- Held game requests: `build-runs/game-queue-halted-1027/`. Lane specs: `build-runs/run7/specs/`.

**Merge queue** (50 entries, `python tools/merge-train.py status`). The credit hold is gone, so many entries now show
READY: none of that is a judgement. Re-judge every entry against this plan and land only explicit batches
(`merge-batch --lanes`):
- Drop the `method-*` entries.
- `trace-replay31`: land only its load-stall fix if its gate passes; no further recorder work.
- Tools that speed the loop (`build-speed32`) and spec answers (`re-*`) can land after a gate.
- `replace-*`, `shadow-*`, `glue-*` and the other engine entries wait for `check` and land by its result.
- Entries that touch the deleted tools (coverage credit, ledger, census replay) need rebasing or dropping.

**Known state.**
- `effectsres` is off the replace list after a start-up crash (EF28); spec question re-ef32 is answered.
- `git push` reports that the repository moved to https://github.com/tloriato/OpenSC4.git; pushes still work.

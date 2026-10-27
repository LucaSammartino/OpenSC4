# Next steps

The one pick-up document for the next coordinator session. It states what we do next and nothing else. Every write
replaces the whole file: no history, no logs, no "previously". Evidence lives in git, in `docs/lanes/` and in
`build-runs/`, never here.

State as of 2026-10-27: **run 7 halted** on the user's order. Only the census still runs. No lanes, no trains, no
operator, no conductor. Nothing restarts until the user approves the experiments below.

## 1. Why we halted

- The house-city "verified" number (31.4%) measures one city with no actions and no UI. It is not a measure of the
  game, so lanes optimised the wrong target. An overnight run of 12 lanes moved it about 0.5 points.
- We never defined, built or optimised a feedback loop. Each check is a full game run (shadows slow the game
  20-80x), so a change costs an hour to judge and the verdict is noisy.
- The operator sub-agent layer made things worse. From here on the coordinator runs about 9 lanes directly, with no
  operator layer.

## 2. Agreed definition of verified (the user said yes)

**Verified** = our output is exact across the whole corpus (census cities, action scripts, UI recordings), counted
only where the corpus reaches. Reach is shown as the ceiling next to it. The house-city number is retired; don't
quote it.

## 3. Facts the plan rests on (measured)

| Fact | Number | Source |
|---|---|---|
| Game speed, plain or light runs | 10-37 game days/s | last 400 runs in `build-runs/game-queue.log` |
| Game speed, full shadows | 0.1-0.6 game days/s (20-80x slower) | same; the reason the census takes about 5 h |
| Replay, 10 cities × 3 owners | 13.5 s cold, 3.8 s cached, 20 workers | trace-replay30 (landed) |
| Replay agreement with the game | 45/45 city/key pairs agree; 36 strict exact | trace-replay30/31 |
| Trace size | 70x zstd+dictionary (42 GB → 0.6 GB); 21-126 MB per scenario for 5 keys | trace-replay31 |
| Recording overhead | **14.3x** (Aquaton: 170.3 s vs 11.9 s control) | trace-replay31; this is the blocker |
| Gate (build + tests) | 409 s per gate after build-speed31; median per change about 125 s | train290 |
| SC4 nondeterminism | 3 identical original runs: about 140 save records differ (residential spread about 2,000), even with `--clock fixed` | golden27 (`tools/golden.py determinism --repeats 3`) |
| Known nondeterminism causes | frame-paced idle cycles (growth developers, aura, plumbing); restart-when-finished passes (demand, landvalue, crime, power, BDS); clock-seeded RNGs; others not isolated | `docs/ARCHITECTURE.md` |
| Machine budget | about 86k CPU-s/h available; 423 GB disk free | |

## 4. The loop we are building

Two loops. The game is used only to record and for a nightly end-to-end run.

- **Inner loop (per change, minutes):** build, then replay our core against recorded original traces for every
  corpus scenario the change touches. Replay about 1 min plus build about 2 min, so **3-8 min per change**.
  8 lanes × 2 checks/h costs about 38k CPU-s/h of the 86k available.
- **Recording (once per corpus change):** record the original's ABI callback answers and outputs for the whole
  corpus. With overhead under 2x, the full corpus records in about 45-60 min on 6 slots. Traces take 20-100 GB.
- **Nightly:** end-to-end replace runs over the corpus in the game, plus a scenario-suite rotation.

Corpus: the 179 census scenarios, action scripts, and UI recordings (including sessions the user plays and records).

Two ways to handle nondeterminism, both kept:

- **Fix A: record/replay at the ABI boundary.** Replay feeds our core the original's recorded answers, so the
  original's nondeterminism is frozen into the trace. Already works for budget, power, pollution, traffic and
  terrain. Blocked only by the 14.3x recording overhead.
- **Fix B: a deterministic original, by in-memory DLL hooks (never an exe file patch).** Hooks give a virtual clock,
  fixed per-day work budgets for the frame-paced and restart passes, fixed RNG seeds, a deterministic allocator and
  `-CPUCount:1`. Then original and replaced runs advance in lockstep and we compare a digest of the city state every
  game day. On the first divergent day, bisect owners from a snapshot (about 5 runs for 30 owners). This also covers
  owners that Fix A cannot record.

## 5. Experiments (waiting for the user's go-ahead; start nothing before it)

| | Question | Pass condition | Time box | Who |
|---|---|---|---|---|
| **E1** recorder overhead | Can recording be cheap enough to record the corpus? | under 2x overhead per game day on Big City and Aquaton (matched DLL and window) | 4 h | one lane, spec `trace-replay32` (game thread appends raw records to a preallocated ring; a writer thread streams them to disk; compression, dictionary and dedup move to an offline `fc4-replay pack`) |
| **E2** determinism | Can hooks make the original deterministic? | 3 identical runs give identical daily state digests for 60 game days | 6 h | coordinator, directly |

If E1 passes: record the census and switch the metric to corpus verified by replay. If E2 passes: build lockstep
digest comparison and owner bisection. If both fail: report the numbers to the user before any other plan.

## 6. What the user does (5-minute checks)

The user takes any check that costs the coordinator about an hour once and the user about 5 minutes: comparing UI
screens, clicking through dialogs, playing and recording a short UI session for the corpus. Give exact steps and
what to look for.

## 7. Picking up from the halt

**Census (the only live work).**
- census-1027 runs on main `d7cf38cb8` with `python tools/suite.py resume census-1027 --slots 6 --wait-seconds 30000`.
  It writes to `build-runs/run7/census1027-resume.out`.
- When it finishes, run `python tools/coverage.py --suite census-1027`. Record the per-key strict baseline here,
  replacing this bullet.

**Saved work.** All of it is local; nothing was pushed or deleted.
- 51 branches `halt-1027/<worktree>` hold every lane worktree's work.
- Held game requests are in `build-runs/game-queue-halted-1027/`. Move them back to the queue only if their lane
  restarts.
- Lane specs are in `build-runs/run7/specs/`.

**Merge queue at the halt.** Re-check each against the new definition before landing; none lands automatically.
- READY:
  - `trace-replay31`: needs its load-stall fix.
  - `build-speed32`: tools; phase deadlines and reaping of hung fc4verify.
  - `re-hg32`: train296 was mid-regress when it was killed; rerun it.
  - `method-html31`, `views-cities29`, `replace-gzfw29`, `automata-cities29`.
- Several others are on hold with "coverage credit missing".

**Lanes killed mid-round** (work is on their halt branches):
- shadow-mysims32, shadow-occupants32, shadow-city32 and shadow-agents32: blocked on spec questions PN4/WC13
  (re-pn32).
- replace-cfgrules30, replace-netocc32, replace-formats32, replace-tools32.

**Known state.**
- `effectsres` is off the replace list after a start-up crash (EF28). Spec question re-ef32 is answered and waiting
  for an implementer.
- `git push` prints that the repository moved to https://github.com/tloriato/OpenSC4.git. Pushes still work; update
  the remote when convenient.

## 8. Order of work once approved

1. Run E1 and E2 in parallel (one lane plus the coordinator).
2. Land `build-speed32` and `trace-replay31` (after its fix). They feed the loop directly.
3. Build the inner loop per section 4. Rewrite `tools/coverage.py` verified to the corpus definition, and delete the
   house-city metric.
4. Only then restart engine lanes (about 9, no operator layer). Each lane names the corpus keys it will make exact.

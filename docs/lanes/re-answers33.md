# Lane: re-answers33 (RE answers: automata question pool AT6-AT19)

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| re-answers33 (RE answers: automata state questions AT6-AT19, 14 questions, docs only) | `C:\Users\tlori\FreeCity4\.claude\worktrees\re-answers33` / branch `re-answers33` | **ready, not merged** (docs only: `automata.md`, `QUESTIONS.md`, `questions/automata.md`, `questions/answered/automata.md`; **no `engine/` touched**, no game runs, no ABI rows proposed). All 14 questions answered from the export with `re_query.py` plus REA Ghidra pseudocode for the two functions the export alone could not settle (`ImportBehaviorTable` `0x00522860`, `DoRandomWalkTick` `0x0050CF90`); every numeric claim checked against instruction bytes. Verdicts: MATCHES x5 (AT7, AT8, AT11, AT14, AT15-in-part), DIFFERS x8, PARTLY x1 — see below. `questions.py check` passes; index regenerated (automata 0 open / 37 answered, next free AT20/AM13/A5). | 1. Merge into main with `git merge --no-ff` (docs only; nothing to gate). 2. The spec's new "slots past 69" table (§11) names automaton slots 77/78/80/82/102/105/125/160/163 — a future ABI lane can lift them into `docs/abi/interfaces.tsv` if wanted (not proposed here; the lane writes none). 3. Nothing else pending. |

## Verdicts, one line each

- **AT6 (PlayerDrive `+0x28`) — DIFFERS.** It is the **drive command accumulator** (speed units per second):
  `cSC4Vehicle::TranslatePlayerDriveCommands` (Mac `0x000a336c`) integrates the throttle/brake keys into it
  (`+0x28 += dt_s × params+0x10/0x14`, sign by forward/reverse, zeroed when both channels are inactive) and
  `cSC4Vehicle::PlayerDriveUpdateSpeed` (Mac `0x000a3e1a`) integrates it into the speed `+0x2C`
  (`vel += dt_s × +0x28`, clamped to the params' max/min). The "never touched by `DoPlayerDriveTick`" reading
  was right about that one function and wrong about the pair.
- **AT7 (Bumped 4 × vec3) — MATCHES** the leftover-record guess: nothing reads them. Mac
  `tAutomataStateBumped` is referenced only by its ctor (zeroes), `Read`, `Write` and the factory case `0x11`;
  no code pushes type `0x11` on either build; `HandleCollision` writes the orientation `+0x58..+0x60` and pushes
  COLLIDE (`0x14`). Windows `cSC4Vehicle::HandleCollision` `0x0055C8C0` confirmed from bytes. Quirk 13 rewritten.
- **AT8 (PlopJump units) — MATCHES**, with the draw's unit named: **km/h**. `U(30,50)` (doubles
  `0x00A95708`/`0x00A95710`) `× 1/3600` (double `0x00A95700`) = 0.0083–0.0139 m/ms; gravity
  `−5.5555563e−05` m/ms² (lazy `0x00B22740`, guard `0x00B22744`, seeded `MOV [0x00B22740],0xB8690455` at
  `0x00505606`); a hop peaks at 0.62–1.7 m over 150–250 ms. §6.2 row updated.
- **AT9 (riot leader pop) — DIFFERS.** Nothing pops the `final` RiotLeader. `cSC4DisasterInstanceRiot::Update`
  (`0x0057DFE0`) ends a leader when its occupant reports `IsFlagSet(1)` true: the no-op `SetState(1,…)`, the
  script-system and disaster-layer unregisters, `Release`, unlink. The automaton is released while still in
  state 8 with `final` set; DISAPPEAR never runs. §8.1 extended.
- **AT10 (`random_walk.turn` offsets) — DIFFERS: no off-by-4.** `ImportBehaviorTable` (`0x00522860`) writes
  the array/scalar into `+0x58` (amplitude) / `+0x5C` (bias) — the same offsets `DoRandomWalkTick` reads
  (`FLD [EBX+0x58]` `0x0050D139`, `FLD [EBX+0x5C]` `0x0050D187`). The `+0x54/+0x58` reading mis-simulated the
  frame; the ctor defaults pin it (`tBehaviorTemplate::tBehaviorTemplate` `0x0050BD40`:
  `+0x58 = 0.1` `0x3DCCCCCD`, `+0x5C = 0.2` `0x3E4CCCCD` — the spec's "0.01, 0.2" default row was also wrong
  and is corrected). A scalar `turn` sets amplitude only. Bonus: the whole walk/idle countdown contract is now
  written under the Lua-key table (two independent seconds countdowns `+0x34` turn / `+0x38` walk-idle; the
  walk never rests when `idle_delay` is (0,0); the rest flips to `−U(idle_time)` and calls
  `TweenSpeed(0, deceleration)`).
- **AT11 (emergency 100–103 names) — MATCHES** the slot mapping, with the naming evidence upgraded: dispatch
  states 0–6 come from the shipped debug strings (`GetDebugExtraText` `0x00532290`, jump table `0x00532414`,
  strings `0x00A98574`…`0x00A98514`), and the state types are tied by the pairings
  `SetDispatchState(3)+SetState(100)`, `NotifyCallbacks(5,0)+SetDispatchState(5)+SetState(0x66)`,
  `SetDispatchState(1)+PushState(0x65)`, `SetDispatchState(6)+PushState(0x67)`.
- **AT12 (`FUN_005096B0`) — answered.** It is **automaton slot 105** (`cSC4Pedestrian::vftable_2` slot 146);
  the emergency family overrides that class-vtable slot with `0x0052F730` (skip when flag `0x80` or top state
  `0x34`). Contract: flag `0x4000` guard; group template lookup (stored at `this+0x98`); queue-info + strength
  draw; walk the top state — **`state+0x24`** equal to the found template ⇒ stop (no push); the **controller
  `state+0x08`** null or group-mismatched ⇒ `PopState`; otherwise `PushBehavior` (slot 55). New §6.5 subsection.
- **AT13 (crash-state fate) — DIFFERS:** nothing reaps a beached/sunk boat. All 41 watercraft functions
  scanned; no state-70/71 cleanup anywhere. The boat sits in its final state until flag-1 removal,
  `HandleOccupantRemoved` (`0x00574520`), or a new `SetState`.
- **AT14 (idle budget) — MATCHES:** exactly two writers across all watercraft functions — `Init`
  (`0x0056DE40`, zero at `0x0056E2A0`) and `DoStateTick` case 7 (`+= dt`; `> 60000.0` `0x00A9DA40` →
  `SetState(0x49 SLOW_REMOVE)`). Lifetime-cumulative by design.
- **AT15 (slot-59 guard) — DIFFERS:** the read is the **player-drive test**; the `== 0x10` comparison is in
  the bytes of `0x00568570` (Ghidra's older decompile dropped it). Player-driven boats get the crash state;
  every other boat gets `SLOW_REMOVE` immediately.
- **AT16 (AI_PursueGoal fall-through) — DIFFERS:** no fall-through. Byte table `0x0053C5A0` over jump table
  `0x0053C57C`: `0x32` has its own handler `0x0053C441` (slot 25 with 0, then pop); the default block
  `0x0053C437` (types 1–`0x31` and > `0x38`) is `PopState` only (`8B 06 8B CE FF 90 D4 00 00 00`).
- **AT17 (slot 102/77/82) — DIFFERS on two of three names.** Slot 102 (`0x0050B540`) posts message
  `0x9D92FC3` to the `this+0x188` callback list with `this` as sender and two payload dwords —
  `NotifyAutomatonEvent(u32, u32)` *[inferred]*; the crash call's second argument is the **crash duration**
  (`0x0056E40B`), not an "anim kill" code; slot 80 (`0x00505860`) wraps it with `(6, 0)`. Slot 77 is
  **`EnableActiveSound(bool)`** (named by the export; Mac `0x000bfba6` agrees) — not `EnableMotion`. Slot 82
  (`0x004FB340`) is **`GetFirstModelInstance`** — not `SetTransform` (Mac `0x000c1c68` is a different body).
  New "slots past 69" table in §11 (77/78/80/82/102/105/125/160/163).
- **AT18 (blocked-wait exit query) — DIFFERS from both guesses:** the query is
  `cSC4City::GetTrafficNetworkMap` (city slot 40) then **`cSC4TrafficNetworkMap::CheckIntersection`**
  (`cISC4TrafficNetworkMap` slot 33, `0x004B06F0`): it returns the **count of automata occupying the
  automaton's approach lane** (`(facing + 2) & 3`) and writes the wait delay to the out `f32` (`FLT_MAX` when
  nothing found). 0 = lane free ⇒ the wait pops.
- **AT19 (`+0x94`, slot `0xA0`) — PARTLY.** `+0x94` is written as `|Δpos|/dt` **only by
  `cSC4Vehicle::DoPathTick`** (`0x0055CFBC`) — the traffic `DoPathTick` has no `+0x94` store; the pull-over
  tick zeroes it (`0x0055E95C`) as claimed; no tick reads it (`GetSpeed` slot 34 `0x004FB370` reads the speed
  tween, not `+0x94`); the one consumer found is `cSC4Train::HandleCollision` (`0x00558E00`), which copies the
  train's `+0x94` onto its cars *[inferred: derived-speed debug field]*. Windows slot `0xA0` (`+0x280`,
  `0x0054DA50`) **is `ReturnToDefaultSpeed`**: duration `1.0/maxSpeed × 0.3` (0.1 when the max is 0), then
  `TweenSpeed(defaultSpeed, duration)` — matching the Mac `0x000caa42`.

## Method note

`re_query.py` answered asm/strings/vtables/callers for everything numeric; the REA Ghidra session (shared, one
`open_binary` at the start, no `close_binary` by this lane except the two accidental `binary_session` probes)
supplied pseudocode for `ImportBehaviorTable` and `DoRandomWalkTick`, whose decompilers resolve the stack
frames that hand simulation got wrong. The `binary_session` call returned a 243 KB dump once; the lane then
used `jq`-style python extraction on the saved tool-result file instead of re-calling.

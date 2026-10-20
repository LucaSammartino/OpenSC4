| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue19 (the automata layer glue: shadow, riot/fire host tables, U-Drive-It, the GZCOM classes) | `glue19` / (engine lane) | **landed** (train10; the lane's own report follows) | landed; AT20 filed (docs/specs/questions/automata.md) |

| glue19 (the automata layer glue: shadow, riot/fire host tables, U-Drive-It, the GZCOM classes) | `glue19` / (engine lane) | **ready** (no C ABI version bump; gate glue19-fix: cargo 2863/0, clippy clean, plugin build clean, ctest 57/57, ret_channel_check 0 flagged after `cISC4CitySituationManager` slot 32 joined SPEC_FACTS; runs 193-197 clean/ended, `ref_warnings=0`): the **automata shadow** (`automata_shadow = 1`) stays exact — run `glue19-shadow-4` (73/73 lines, 774,664 bytes, 0 mismatches) and, on the fix-round tip, the save / reload pair: run `glue19-save-196` `SaveCity(full)` inside the exact shadow (358/358 lines, 3,837,340 bytes) and run `glue19-reload-197` the saved 2026-02-13 state reloaded with the shadow exact (117/117 lines, 1,257,036 bytes; `Read: version 4 ... julian 2461085`), so the records round-trip a save. `FC4AutomataDriver` now also bridges the **fire** half: `AutomataDriverFireSetTarget` / `_Shutdown` wired to the disaster host's live `create_fire_occupant` / `remove_fire_occupant` (the occupant itself is still Q27, so the bridge is wired and inert), and **U-Drive-It** pause / resume is wired: the observer subscribes the city situation manager's activation message `0xABBBCA64` (data1 = 1 pause through `AutomataPauseAll`, slot 9; data1 = 0 does not resume, CS6) and its tick polls the manager's `CountdownIsInProgress` (slot 32) and resumes through `AutomataResumeAll` (slot 10) when the countdown that ran has ended (city-situation.md 4.4 steps 1-2); the driver logs its host counts at the city shutdown. **The riot / fire driver stays unexercised in game on this branch**: the disaster layer is the original's (there is no `enable = disasters`), so its `create_riot_leader` never reaches the FC4DisasterHost table the driver serves — runs `glue19-riot-1` / `-2` show a scripted riot starting (`AttemptDisasterStart(40, 33)`, `active=1 ongoing=1`, the game's own disaster emergency pause holding the date) with `leaders created 0, ... fire controller calls 0` as expected. `automata_shadow = 1` joined the regress shadows list. Questions AT20 filed (the receiver names of the pause / resume) | Next: (1) an RE answer to AT20 (what city-situation slots/messages reach the four managers' Pause/Resume in the original; the wiring waits for it — `AutomataDriverSituationMessage`'s message trigger and `_Tick`'s countdown poll are the *[inferred]* reading, marked in the code); (2) exercise the driver in game when a disasters replacement lands (its `create_riot_leader` then hands the leader to `fc4_automata_riot_add`, `_set_stage`, `_end` and `fc4_automata_fire_set_target` / `_shutdown` through `AutomataDriverRiotAdd` / `RiotSetStage` / `RiotEnd` / `FireSetTarget` / `FireShutdown`); (3) the month-agent slot-83/89 vtable answer (AT20 of the fix round, an RE pass over the four managers' primary vtables) before `enable = automata` can run a played city; (4) the enable run after that; (5) merge: the train may need the FC4Subsystems.cpp union (the branch's automata row is at the end of `kSubsystems`) and the fc4.h history comment. Questions AT20 (this round) and the earlier A1-A4 filed |

## Report

### What changed (commits on the branch, newest first)

- `a28c7282` **automata_shadow in the regress shadows list** (the fix round's step (6)): `tools/regress.ps1`'s shadows
  profile ends with `automata_shadow = 1`; the header comment names it.
- `1dc7cfbd` **the driver's shutdown counts line**: `automata driver: off: leaders created N, refused N, SetState
  calls N, stage-rise calls N, fire controller calls N` at the city shutdown, from `FC4AutomataHost`'s counts.
- `9837af8f` **the fire bridge and the U-Drive-It pause / resume** (the fix round's step (3)):
  - `FC4AutomataDriver.{h,cpp}`: `AutomataDriverFireSetTarget(token)` / `AutomataDriverFireShutdown()` over the
    driver's `fc4_automata_fire` (the 8.2 bridge; the fire occupant itself is still Q27 on this branch, so the
    bridge is wired and inert); the U-Drive-It state (`udrivePaused`, `countdownSeen`),
    `AutomataDriverSituationMessage(data1)` (the pause on `0xABBBCA64` data1 = 1, through
    `fc4::AutomataPauseAll` = manager slot 9 on the four managers) and `AutomataDriverSituationTick()` (every
    framework tick: polls `cISC4CitySituationManager::CountdownIsInProgress` (city slot 49, slot 32) and resumes
    through `fc4::AutomataResumeAll` (slot 10) when a countdown that was seen running has ended — the spec's own
    resume, city-situation.md §4.4 step 2; CS6 says a deactivation does not unfreeze, the countdown running out
    does). The message trigger is *[inferred]* (automata.md §9 names only the manager slots; the activation
    message is the observable), marked in the code and asked as AT20.
  - `FC4DisasterHost.cpp`: live-mode `remove_fire_occupant` hands the removal to the driver (the Shutdown of 8.2)
    when the fire token is a real object; `create_fire_occupant`'s comment names the bridge.
  - `FC4Observer.cpp`: `kMsgSituationActive` (0xABBBCA64) subscribed with the other observer messages and routed
    to the driver; the driver's tick runs after the shadows' in `OnTick`.
  - `engine/tests/check_ret_channels.py`: `SPEC_FACTS` gains `("cISC4CitySituationManager", 32)` (city-situation.md
    §8: CountdownIsInProgress, EAX 0 or 1), so the plugin's bool read is reviewed rather than flagged.
- Earlier (already on the branch at resume): the automata shadow, the state-record corrections, the host tables,
  the six GZCOM classes (registered, `enable = automata`, off by default), the QueryInterface create-path fix
  (A4), the wrappers and the riot half of the driver.

### Measured results

- **Gate glue19-fix** (at `9837af8f` + the counts line `1dc7cfbd`, `coord-verify-lane.ps1`): configure 0, build 0,
  ctest **57/57** (the first run flagged `CountdownIsInProgress` in `ret_channel_check`; fixed by the SPEC_FACTS
  row, rerun clean); **cargo 2863/0** (`cargo test --workspace --target i686-pc-windows-msvc`, the glue19
  baseline); **clippy clean** (`-D warnings`); rustfmt: no `.rs` files touched, no diffs.
- **Run 193 `glue19-riot-1`** (Konradshohe, `automata_shadow = 1`, `disaster survey` + `disaster riot auto`):
  clean, `ref_warnings=0`, the automata shadow exact (1 line). The riot started
  (`AttemptDisasterStart(40, 33) -> instance 00000000; active=1 ongoing=1`) and the game's disaster emergency
  pause held the rest of the run at 2026-02-02.
- **Run 194 `glue19-riot-2`** (the same + the counts line): clean, `ref_warnings=0`, the shadow exact;
  `automata driver: off: leaders created 0, refused 0, SetState calls 0, stage-rise calls 0, fire controller
  calls 0` — the expected empty count with the original disaster layer (see below).
- **Run 195 `glue19-savesave`** (queued before the fix round, `243f82ff`, one `save` at 2030-01-01):
  ended (the game ended the city itself, 0 problems), `ref_warnings=0`; `SaveCity(full)` ok in 10,766 ms; the
  shadow exact for 464 lines (2,790 components, 4,929,327 bytes, 0 mismatches).
- **Run 196 `glue19-save` / run 197 `glue19-reload`** (the direct `ingame_run` pair with `-KeepSaves`, because the
  queue's region mirror undoes a slot save between runs): save clean (`SaveCity(full)` in 10,969 ms, shadow
  exact 358/358 lines), reload clean (`Read: version 4 ... julian 2461085`, the saved 2026-02-13; shadow exact
  117/117 lines, 1,257,036 bytes). **Save -> reload round-trips under the automata shadow.** Two earlier pairs
  (g19sr, g19sr2) were diagnostic: no `autoload_city` (no load), then no `-KeepSaves` (the restore undid the
  save before the reload).
- The test region was restored to its original `864E0A88` state after the pair.

### The riot / fire driver's in-game status

The driver is reachable only through `FC4DisasterHost`'s host table, which serves **our** disaster core
(`fc4_disasters`): in shadow mode the mirror drives it (the original's leaders handed over), in live mode ours
would — but there is **no `enable = disasters`** on this branch (glue13's replacement never registered a
subsystem key), so the live path cannot fire. The scripted riot of runs 193/194 went through the original layer
directly (its own `CreateRiotLeader`), which is why the driver's counts stayed 0. The wiring is therefore
complete and ready for the day a disasters replacement lands; exercising it in game needs that replacement
(next steps item 2).

### Questions filed

- **AT20** (`docs/specs/questions/automata.md`, the QUESTIONS.md index updated): the receiver names of the
  U-Drive-It pause / resume — what city-situation slots or messages reach the four managers' Pause / Resume in
  the original, and when exactly. `AutomataDriverSituationMessage`'s message trigger and `_Tick`'s countdown
  poll are the *[inferred]* reading, marked in the code.
- Earlier: A1-A4 (the fix round's A4 answered by the QueryInterface fix) and AT20 of the fix round (the four
  managers' primary vtables past slot 37) were already on the branch.

### Next steps

1. AT20's RE answer, then re-check the wiring against it.
2. Exercise the driver in game once a disasters replacement exists.
3. The four managers' vtable RE pass (the fix round's AT20) before `enable = automata` survives a played city.
4. The `enable = automata` run after that.
5. Merge: the FC4Subsystems.cpp automata row sits at the end of `kSubsystems` (union-friendly); `fc4.h` was not
   touched by the fix round.

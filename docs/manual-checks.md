# Manual checks backlog

Things only a human can do or decide. Newest at the top of each section. Claude keeps this updated.

## Needs you (blocking)

Nothing right now. The game opens to the region view, so the plugin is getting an `autoload_city` option; after
that, bisection runs need no clicks.

## Budget corruption bisection

| Run | Switches | Result |
|---|---|---|
| 8 | all on, message-layout fix | corrupt: NT heap unlink AV right after load. Cause: `GetAllGroups` pushed a vector element holding a string by bitwise copy (double free). Fixed in 8d79c77 (merged). |
| 9 | all on, with the fix | load and first month OK; crash at month end in the game's sound-event pool (exe+0x1E16F5) |
| 10 | `message_posting = 0` | same month-end crash, so messages are not the cause |
| 11 | `lot_scan = 0` | **clean** for two months: the culprit is the lot scan (`ForEachLot`) |
| next | lot_scan sub-switches | in progress: the plugin lane is adding sub-switches and auto-load |

## Progress log while you were away

- M2 core (demand, ordinance, weather) merged, a164fc9. Whole workspace test suite green.
- Specs committed:
  - dbf6237: flora spec; foundations §4.1/§5.2 fixes; M3 open questions resolved.
  - 2412ca3: ABI rows; flammability prop/flora label erratum.
- Implementer questions are queued in `docs/specs/QUESTIONS.md`.
- Budget heap corruption, static lead:
  - Our message object has type and flags at swapped offsets compared with the game's (`docs/abi/layouts.md`).
  - A fix was requested from the plugin lane.
  - This is unproven; the bisection still decides.
- Specs committed:
  - b9da700: fire and police protection.
  - ebc9e88: plumbing.
- Land value simulator and the game-layout message fix merged, e2dc324. Land value is off by default
  (`enable = landvalue`).
- The checked DLL with the message fix is staged in Plugins. The previous one was renamed `*.pre-msgfix`.
- Run 8 starts automatically once you close the game: all budget switches on, with the message fix.
  - If it's clean, the message layout was the cause.
  - If not, the bisection continues.
- Specs: power (b06e048). All open questions are answered (80d86eb … 0d45db2).
- New foundations §18: the simulation runs at 24-bit x87 precision. The specs' worked examples were recomputed
  (cde56f4).
- M3 cores merged (4e7ccbb): grid module, pollution, crime, flammability and aura. 181 conformance vectors and
  155 core tests are green.
- Now running:
  - lane 1: budget test 14 recheck, then the in-game glue for weather, ordinance and demand (off by default);
  - lane 3: grid reconciliation, then the cores for power, plumbing, fire/police and flora;
  - spec lane: education, health and mayor rating.

## To verify in-game (after the bisection finds the culprit)

- [ ] Budget replacement, all switches on: 2–3 months at cheetah speed, money changes each month, budget
      window (income, expenses, departments) looks normal.
- [ ] Save → exit → reload with our simulator and budget: the date, funds and departments survive.
- [ ] Pause, unpause and all three speeds. Emergency pause, if a disaster can be triggered.
- [ ] Cheats: `RiskyMoney`, `WeaknessPays`, `Moolah` (if enabled) do what the original does.

## Decisions for you

- [ ] Upstream to gzcom-dll: the SDK has several wrong signatures and return types (see `docs/abi/` and
      foundations §17 errata). Report them to the SDK maintainers? (Public action, so it needs your OK.)
- [ ] Repo hosting and publication (GitHub, project name, README wording) when you want it public.

## Done / answered

- 2026-10-12: language question → logic stays clean-room; interface facts (`docs/abi/`) are open to
  implementers.

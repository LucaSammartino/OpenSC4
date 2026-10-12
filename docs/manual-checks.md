# Manual checks backlog

Things only a human can do or decide. Newest at the top of each section. Claude keeps this updated.

## Needs you (blocking)

1. **Close the running game** (Task Manager → SimCity 4 → End task). It was started elevated, so Claude can't
   close it. After that, Claude launches the game from its own shell (normal integrity), so it can stop and
   relaunch it alone and run the budget-corruption bisection unattended.
2. **Confirm the game opens the last city by itself** at launch. If it opens to the region view instead,
   unattended runs need one click per run, or a load-on-start option.

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

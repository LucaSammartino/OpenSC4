tag: rbc7f-86
run: 86
date: 2026-10-14
lane: rbc7f
dll: 2f5a507a
pacing: unknown
verdict: PASS
switches: main `bc7ffa0` (the residential glue; DLL `2f5a507a`), `tools/regress.ps1 -Tag rbc7f` with `residential` added to the replace list and `residential_shadow = 1` in the shadows run (Konradshohe): **first in-game run of our residential simulator and of its shadow**
result: **PASS, 0 problems.** Replace (fourteen replacements): clean 150 s, 2026-01 → 2033-06; `residential month` lines every month: population 15,025 → 24,462 (R$ 22,494, R$$ 1,751, R$$$ 217), EQ 58.0 → 38.1, HQ 68.6 → 42.7, life expectancy 67 → 60; the original declines the same way (EQ 42.2, HQ 50.2 in the shadow by 2030, equal on both sides), and run 86's replace run reaches EQ 48.6 / HQ 54.7 at 2030. Save Write/Read lines: not exercised (no save in regress). **Shadow: 661 of 881 lines exact**; each cycle 12 of 15 daily states match on all 25 grids, the saved fields, the 24 per-building properties, funding and the scalars (population, EQ, HQ, life expectancy identical to the last digit). States 1–3 (the proximity grids) differ in about 16,000 of 16,384 cells: the original's corner reads 1, 0, 0, 0 where ours counts down 104, 103, … (RG23). The residential shadow slows the game: the shadows run covered 57 months in 150 s against 95. Budget shadow 30/31: one month's total is 1 above the original's with every tax line equal (BQ3). Others: demand 1737/1737, land value 57/57, protection 112/112, landfill and pollution 56/56, plumbing 108/113, power 265/321 (PW8), flora 42/56 (FQ15), aura 4/57 (AU13–AU15).

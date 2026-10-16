tag: run-69
run: 69
date: 2026-10-14
lane: run
dll: unknown
pacing: unknown
verdict: ended
switches: main `36ababe`, soak 2: `enable = landvalue, weather, ordinance, demand, power, plumbing, flammability, pollution`, `popup_dialogs = 0`, `observe = 1`, quit, 900 s
result: **no verdict: the game ended the city itself** (2057-10, 10.5 min in; funds 891 → −103,047). 0 foreign exceptions, `ref_warnings=0`, every Shutdown line. Power and plumbing `enabled=1` all 381 months. The city grew until 2051 (R 15,025 → 30,884) and collapsed from 2054-03 (R 25,066 → 10,783 in one quarter, 866 by 2055-07, industry 0). By then both utilities were short: power production had fallen 7,110 → ~5,800 (aging plants, no player to replace them) against a requirement of 11,872, with 195–258 of 568 consumers unpowered; water pipe condition was 0.000 and production (19–37k) short of demand (43.5k), with 0–492 of 572 building cells watered. (Consumption = 110 % of production is the spec's shortfall rule, power.md §6 step 4, and `deficit=0.000` its quirk.) Run 61 kept growing only because power and plumbing were disabled there. Whether the original simulators decline the same way is open (runs 71/72, power shadow lane). Two follow-ups: the autotest now gives verdict `ended` when the game shuts the city down by itself (main, after this run), and a soak with the original power simulator for comparison.

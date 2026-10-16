tag: run-20
run: 20
date: 2026-10-12
lane: run
dll: unknown
pacing: unknown
verdict: clean
switches: our budget, original simulator, observer
result: clean. **Differential vs run 19, first month** (identical start): health and education expense +1 each (123/122, 105/104); tax by type differs in types 4, 6 and 9 (total income 947 vs 944). Investigated (f59e085): **ours is right**. The real game gives 123/105, as the year-to-date records of the unmodified save show, because it runs x87 maths at 24-bit precision. Run 19 ran at 53/64-bit, so something in the test process reset the precision. That may be our DLL, which would make every original subsystem compute wrongly whenever FreeCity4 is loaded. Being instrumented by the plugin lane. The tax-by-type difference is still open (agent order, precision, or the lot set); a per-lot dump is requested. **[Resolved 2026-10-13: it IS our DLL — see row 24 and foundations §18.2 (ea602a4, e647189). The precision conclusion in this row stands, and ours is confirmed right.]**

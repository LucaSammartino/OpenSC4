tag: run-33
run: 33
date: 2026-10-13
lane: run
dll: 33FC0F9B…
pacing: unknown
verdict: clean
switches: main `1eb5dd6` (DLL `33FC0F9B…` with its own PDB), `enable = plumbing, flammability`, Spandau
result: **clean**. Plumbing passes run (23 → 101), building cells 396 → 412, no re-entry. But Spandau has **no water system** (`producers=0 pipes=0`), so only the empty-network path ran, and flammability logged nothing after `Init`.

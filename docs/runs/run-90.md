tag: run-90
run: 90
date: 2026-10-14
lane: run
dll: d3ca1ef3
pacing: unknown
verdict: clean
switches: main `e21c1a1` (DLL `d3ca1ef3`), `build-runs/soak-orig-resshadow.ps1` (Konradshohe, 900 s, `observe = 1`): **the original's residential path, for run 89**
result: The first attempt used `disable = simulator, budget`. That leaves the residential shadow inactive, because shadows run inside our simulator's delivery, so it gave no residential lines. **Run 90b** (`disable = budget`, `residential_shadow = 1`): clean, 2026 → 2064-11, `ref_warnings=0`, x87 leftovers 0, 0 problems. **The original's EQ/HQ/LE** at 2031/36/41/46/51/56/61: EQ 42.3 / 29.4 / 23.8 / 20.4 / 17.7 / 19.4 / 19.8, HQ 51.6 / 35.5 / 29.0 / 24.5 / 20.6 / 23.5 / 23.4, LE 62 → 55. Run 89's (ours, fourteen replacements): EQ 44.8 / 28.0 / 23.1 / 18.8 / 16.3 / 15.3 / 20.8, HQ 53.2 / 33.4 / 27.5 / 21.6 / 18.0 / 17.7 / 21.8, LE 63 → 55. **Our residential follows the original's decline**, a few points apart, which is within what separate runs give. The shadow's own scalars are identical on every state-0 line, and 5,592 of 7,456 shadow lines are exact; the rest are proximity states 1–3 (RG23). Funds 18.7k / 58.4k / 101.0k / 136.2k / 140.2k / 76.1k at 2029/35/41/47/53/59, inside the spread of runs 84/85.

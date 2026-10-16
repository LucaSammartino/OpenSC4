tag: run-89
run: 89
date: 2026-10-14
lane: run
dll: d3ca1ef3
pacing: unknown
verdict: clean
switches: main `a41ae84` (DLL `d3ca1ef3`), `build-runs/soak-all14.ps1`: fourteen replacements (residential included) plus the default simulator and budget, Konradshohe, 900 s, `observe = 1`
result: **clean, 0 problems, `ref_warnings=0`, x87 leftovers 0, 2026-02 → 2065-05 (473 months)**: the first long run of our residential. Funds 20.0k / 64.4k / 104.5k / 134.4k / 134.1k / 93.1k at 2029/35/41/47/53/59, inside the original's spread (runs 78/85); the city ends at −54,284 in 2065 as it ages. Residential: population 15,025 → about 25,000, then 19,133 by 2061; **EQ 58.0 → 28.2 (2036) → 16.4 (2051), HQ 68.6 → 18.1, life expectancy 67 → 54, R$$$ 0 from 2056**. The original's long-run EQ/HQ is not logged anywhere (the observe line has no residential getters), so this path cannot be compared yet: the observe line should read `GetGlobalEQ`/`GetGlobalHQ` from whichever residential class is active.

tag: mtract-109
run: 109
date: 2026-10-14
lane: mtract
dll: FreeCity4-mtract
pacing: turbo
verdict: PASS
switches: main `76bf685` (tract glue merged, ABI 48; DLL `FreeCity4-mtract`, sha256 `f480a3a7`), `tools/regress.ps1 -Tag mtract -Queue` (turbo to 2032-01-01; tract stays original here: it is not yet in the regress replace or shadow lists)
result: **PASS, 0 problems.** Both runs clean, `ref_warnings=0`; replace 14.97 days/s, shadows 13.68 days/s (both slots ran beside each other). Shadows exact as run 108 or better: demand 2242/2242, land value 71/72 (the one line is 2026-02-07 pass 2, 3 of 4096 intrinsic cells, max

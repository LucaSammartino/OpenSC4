tag: apply10b-111
run: 111
date: 2026-10-15
lane: apply10b
dll: FreeCity4-apply10b
pacing: turbo
verdict: PASS
switches: main `0f3ad91` (apply10b: exemplar EX34/EX35/EX38/EX39, GF7 model-key readers, protection PR17/PR18, landfill LG21/LG23, agent order AO1; ABI 50; DLL `FreeCity4-apply10b`, sha256 `43f2ee90`), `tools/regress.ps1 -Tag apply10b -Queue` (turbo to 2032-01-01)
result: **PASS, 0 problems, 189 s.** Both runs clean, `ref_warnings=0`; replace 24.86 days/s (17 replacement groups; landfill 72 months, zones=1, as before). Shadows 13.11 days/s: development 913/913, tract 145/145, zones 292/292, demand 2243/2243, power 364/364, plumbing 145/145, residential 1141/1141, protection 288/288, landfill 72/72, flammability 54/54 (run 110's hotspot line is gone), land value 72/73 (the usual 2026-02-07 pass 2 early-settling line, 3 of 4096 intrinsic cells, max

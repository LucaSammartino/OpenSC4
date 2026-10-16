tag: apply10a-110
run: 110
date: 2026-10-15
lane: apply10a
dll: FreeCity4-apply10a
pacing: turbo
verdict: PASS
switches: main `b398049` (apply10a: development answers DG4, DG5, DG6, DG11; ABI 49; DLL `FreeCity4-apply10a`, sha256 `56567523`), `tools/regress.ps1 -Tag apply10a -Queue` (turbo to 2032-01-01)
result: **PASS, 0 problems, 199 s.** Both runs clean, `ref_warnings=0`; replace 28.61 days/s (18 replacements, development among them): BDS slot 23 foundations 451/451, wall loads 32/32 (DG6), funds 85,132. Shadows 12.32 days/s, as run 109: development 913/913, tract 145/145, zones 292/292, demand 2242/2242, power 371/371, plumbing 144/144, residential 1138/1138, land value 71/72 (the usual early-settling line), every other shadow exact except **flammability 53/54**: cycle 6 (2026-08-16) has every grid exact (raw, adjusted and the flammable getter 0 mismatched) but a different hotspot set (6/6, other tracts); the hotspots come from the `rand()` shuffle, and the shadows run does not use our development code, so not this merge's.

tag: rneigh2-96
run: 96
date: 2026-10-14
lane: gated
dll: 3ba27299
pacing: unknown
verdict: PASS
switches: main `5e61237` (the `neighfix` merge, lane gated after merging main `c2c59cf`; DLL `3ba27299`), `tools/regress.ps1 -Tag rneigh2` (fifteen replacements, neighbours included; every shadow)
result: **PASS, both runs clean, 0 problems, `ref_warnings=0`.** The neighbours replacement runs to 2033-06 (95 month lines; 7 connections, a ferry created on the first day). Init logs `ReloadCity replaced the regional city at (4, 7) (141D1414 -> 17F21C14); the old object released`, twice, and no warning follows. Shadows: residential 1472/1472, land value 93/93, aura, flora, flammability, demand, budget, protection, pollution, crime, landfill and neighbours all exact. Only power 376/468 and plumbing 173/185 differ (PW8/PS8: apply6b).

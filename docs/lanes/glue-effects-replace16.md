| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-effects-replace16 (registration and creation-slot ABI) | `Codex` / `glue-effects-replace16` (engine lane) | Ready: cargo 7501/0; clippy clean; CTest 85/85; isolated and house replacements clean to 2032; native names 73/73 | Coordinator: review saved house profile and append effects; broader animation/payload verification remains open |

Registration fix carried forward from 72801629b: remove the director's second effects guard; avoid shadow startup GZCOM reentry and duplicate tick subscription.
11dbdb3fb fixes the actual post-Init crash: use distinct virtual names for slots 6 (ID pair, pop 12) and 7 (name, pop 8), preventing MSVC overload-group reversal. Nonvirtual overloads preserve source compatibility.
Removed argument-shape dispatch, which could not repair stack cleanup; failed creation leaves the output untouched per section 4.
Factory CTests exercise raw slots 6/7, registration on/off/default/shadow, identity offsets, rejected queries and shared reference counts.
The final code commit shares the decoded shipped directory with the replacement mirror, removing synthetic-directory creation differences; the core retains its own reference.
Final stable-source qbuild all PASS: Rust 7501/0, clippy 0 diagnostics, native build 50/50, CTest 85/85; request qbuild-20261025-023153-build-ctest.
Final DLL SHA256 prefix: 3d33bd43. All following runs use turbo and have exceptions=0, ref_warnings=0, panics=0 and other warnings=0.
Isolated replacement: glue-effects16-final-iso-r7 clean, 2032-01-16, 27.781 real seconds.
House replacement: glue-effects16-final-house-r7 clean, 2032-01-01, 105.562 real seconds; AddCls registered 0x49822F75 and effects Init ran.
House list copied from the saved r3 request plus effects; clean-room restrictions prohibit reading/executing tools/regress.ps1. Exact profile is in build-runs/game-queue/done/glue-effects16-final-house-r7.req.
Replacement mirrors report differences 0 but ticks 0; these runs establish startup/soak safety, not animation equivalence.
Exact native name result: glue-effects16-final-native-r7 73/73; clean to 2032-01-01 in 116.250 seconds.
Every native load/month sample: entries=1154, queries=3465, refusal=0, mismatches=0; payload/history unavailable. No broader native equivalence claim.
Run evidence: build-runs/game-results/<label>.verdict and <label>.log; requests under build-runs/game-queue/done/. No separate docs/runs record under revised lane rules.
Failed evidence retained: glue-effects16-registered-iso-r5 corrupt after factory_smoke_stack; corrected slots first pass in glue-effects16-slots-iso-r6 (2032-01-13 clean).
Still missing: ID-pair creation resolution, complete collection publication/internal bases, rendering and natural animation/payload coverage. No native signatures guessed.
Questions: none under revised rules. Shadows remain default-off; C ABI exports/version, tools, coverage maps and house profiles unchanged.

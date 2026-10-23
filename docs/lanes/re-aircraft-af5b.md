| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-aircraft-af5b (aircraft flight contracts) | Codex / detached `re-aircraft-af5b` | in progress | Verify AF2 forces/physics, then AF3–AF6; add conformance vectors |

## Handoff

- Goal: answer AF2–AF6 in aircraft.md with instruction-verified numerical rules.
- Read prior re-aircraft-af5 handoff and core-aircraft5 closure report; AF1/AF7/AF8 are complete.
- Next: query airplane CalculateForces and UpdatePhysics export, then selected decompilation.
- No engine files read or edited: owner's lane report supplies unsupported scope.
- Main RE sources: C:/Users/tlori/FreeCity4/decomp/cSC4Airplane.c and cSC4Helicopter.c.
- Prior lane found no REA open target; use export/decomp fallback.
- No ABI changes, game requests or proposed ABI rows.

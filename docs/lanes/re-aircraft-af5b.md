| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-aircraft-af5b (aircraft flight contracts) | Codex / detached `re-aircraft-af5b` | in progress | Verify AF2 forces/physics, then AF3–AF6; add conformance vectors |

## Handoff

- Goal: answer AF2–AF6 in aircraft.md with instruction-verified numerical rules.
- Read prior re-aircraft-af5 handoff and core-aircraft5 closure report; AF1/AF7/AF8 are complete.
- Done: aircraft.md §§3.4.1–3.4.2 give partial AF2 numerical contracts and vectors 8–13.
- Next: validate docs, then finish report; research must stop by 100,000 measured context.
- AF2–AF6 remain open; no question is complete enough to archive. Owners remain unclosed.
- UpdatePhysics pseudocode is missing; candidate integration and ordering checked in export instructions.
- Verified caller reloads converted seconds; pseudocode misleadingly reused millisecond parameter.
- Force helper 009F0090 is inverse cosine, despite TransformDirection export label.
- Remaining AF2: lift-frame/attitude update 004FE050, surface/runway/collision corrections, rotor model.
- No engine files read or edited: owner's lane report supplies unsupported scope.
- Main RE sources: C:/Users/tlori/FreeCity4/decomp/cSC4Airplane.c and cSC4Helicopter.c.
- Prior lane found no REA open target; use export/decomp fallback.
- No ABI changes, game requests or proposed ABI rows.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-aircraft-af5 (AF1–AF8 aircraft contracts) | Codex / detached `re-aircraft-af5` | in progress | Resolve aircraft questions from original-program evidence; doccheck |

## Handoff

- Goal: answer AF1–AF8 in aircraft.md, add conformance vectors, archive answers.
- Done: read rules, owner lane report and the aircraft owner closure README table (only requested engine documentation).
- Done: AF1/AF7/AF8 specified and archived; exact defaults, Lua field names, zero/non-finite/ancestor rules, modulo-32 mask; vectors 4–7.
- Next: AF2–AF6 flight controls/lifecycle/weapons; verify numeric rules in assembly.
- REA batch attempt failed: target_unavailable, no app open; fallback to export assembly and local decomp only.
- Worktree intentionally detached; commit HEAD without switching or publishing refs.
- RE sources: main checkout absolute paths. Never edit engine, plugin, capi, tools/coverage-map.tsv, ABI tables.
- No game runs or ABI changes planned.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-aircraft-af5-r1-manifest.json.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-disasters-boundary4: answer DS42-DS44 (disaster layer replacement boundary) | `re-disasters-boundary4` / .claude/worktrees/re-disasters-boundary4 | in progress | see Handoff |

## Handoff
- Goal: answer DS42 (city creation/IID/refs/shutdown of layer 0x61F6ABF5), DS43 (global 0x00B43D3C, effect/toxic layer ptrs), DS44 (consumer audit: QI 0x452294AA, raw field access) in docs/specs/disasters.md.
- Done: worktree created at main 39d03bc3.
- RE done (static; no REA needed). Facts: city Init 0x45BD20 -> CreateCityComponents (call 0x45BEBF) -> GZCOM slot3 GetClassObject(0x61F6ABF5, 0x29B02A2C, &city+0x1C4) at 0x45B014 only if null; factory 0x578B70 (0xD8 bytes, returns +8, no AddRef), registered by cSC4COMDirector 0x460B57; then SL::OnCityInit 0x45BEC4 writes global from city slot72 (0x601D79, no AddRef); layer Init 0x45C0CD after civic +0x1B0 before demolition +0x120. Shutdown 0x45B1D0: SL::OnCityShutdown first (clears global 0x602056), ... demolition, layer Shutdown slot4 + Release + null (0x45B346-0x45B360), civic.... Not saved (city Read/Write this=city+8, no +0x1BC ref).
- Global readers (raw byte scan, 8): 0x4283A7/0x43DD61 slots16,17; 0x44455D slot7 (null-checked); 0x50499C,0x5572D3,0x561306 slot10; 0x561319 slot7; 0x693578 slot7.
- Effect inst +0x20 / toxic +0x14: city slot72 + AddRef in Init (0x577062/0x57ED34), Shutdown slots 9,14 then Release.
- Instance inits (volcano,quake,pipe,riot,effect,toxic) and plumbing 0x6DD309 cast slot7 result by -8 (manager concrete). build-runs/audit120.txt = 119-site audit.
- Next: write disasters.md sec 2.8 + answers DS42-44, move questions, index.

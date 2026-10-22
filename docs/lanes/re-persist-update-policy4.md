| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-persist-update-policy4: PR4 service Update return | Codex / `re-persist-update-policy4` (RE docs, detached HEAD) | in progress; proof `590ac4f3`; PR4 answer and six vectors written; no ABI changes | Validate docs and commit final report. |

## Handoff
- Goal: answer only persistence PR4; no owner expansion, code, engine, tools or ABI-table edits.
- Done: reused reserved detached worktree; read manager §9 and PR4; export confirms Update `0x0097356D`, size 75, no direct callers.
- Proof milestone: framework dispatcher `0x0087A812`, virtual slot-11 call `0x0087A8C1`, ignores AL/EAX: next instructions advance EDI then compare EDI/EBX (`0x0087A8C4–0x0087A8C9`). No result-driven branch, store or accumulator in this loop.
- Dispatch body snapshots framework list at +0xA8 to a temporary vector, passes counter at +0xC4 to each slot-11 call; target body has data reference `0x00AC30D0`.
- Evidence commands: main `tools/re_query.py asm 0087A812 --bytes --max 56` and `--skip 56 --max 30`; raw export `instructions.asm` confirms one +0x2C call here.
- Native fact: cache results OR into uninitialized `[EBP-1]` at `0x0097359C`, returned at `0x009735AF`; empty also uninitialized.
- Done: proof milestone committed as `590ac4f3` within the initial 15–25-call window.
- Done: manager OnStart `0x0097530F` creates service, registers through framework slots 3/11; framework slot 11 `0x00879D21` appends service to +0xA8 list used by dispatcher.
- Done: raw executable table `0x00AD89C8` -> `0x0097356D`; framework table `0x00AC30D0` -> `0x0087A812`.
- Done: all 19 immediate service-ID references inspected: constructor plus 18 manager-IID lookups. Framework interval has three indirect +0x2C calls; only `0x0087A8C1` is the service Update loop.
- Done: manager §9.1/9.2 now distinguish native byte S OR callbacks from chosen false-initialized logical OR; §10.1 has six independent vectors. PR4 moved to answered/persistence.md; questions index regenerated.
- Next: recheck whitespace, changed paths, question movement and policy/vector consistency; replace Handoff with full report and queue ready docs tip.
- Main read-only RE roots: `C:/Users/tlori/FreeCity4`; WT is the current directory.
- q.py Bash wrapper fails with CreateInstance/E_ACCESSDENIED; use bounded PowerShell/local Python commands.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-persist-update-policy4-r1-manifest.json`.
- Final third persistence-family round: no retry or subagents. Proof commit due by call 15–25.
- No game runs, gate or ABI proposals needed for this docs-only task.

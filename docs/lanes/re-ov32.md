| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-ov32: overlay allocation and draw records; kind docs (RE) | Codex; re-ov32 worktree; reserved branch re-ov32 (detached HEAD) | ready; content commits c3db3577e, 7743711d3; ABI unchanged, no seam reservation; doccheck passed, diff clean; game n/a, shadow n/a; OV18/OV19 answered | 1. Land queued docs. 2. replace-overlays31 can implement the specified routing and native records. |

- Imported replace-overlays31's overlay-manager question file; it already matched the starting revision.
- OV18: exact pre-allocation interval, four allocator/constructor/Init sites, stack/register continuation contract, field/parent reference transfers and matched native heap owner.
- Enumerated direct manager Shutdown calls, field Releases, draw-subobject registration/unregistration and the view-destructor residual-release loop; included a worked replacement lifecycle.
- OV19: all 36 combiner bytes including tail/padding zeros; complete 14 packing arguments for each of five states, explicit culling and preservation masks, normal/debug and state-replay worked cases.
- Answers are in overlay-manager.md §§3.8.1/4.1/4.8; archived OV18/OV19 and regenerated the question index. No new questions.
- Evidence: original export queries and class decompilation, numeric/width/register claims checked against instruction bytes; no REA calls or runtime execution.
- Validation: doccheck check passed; git diff --check passed. Cargo/clippy/ctest and game/shadow checks n/a for this docs-only lane.
- questions.py check still reports 14 unrelated existing errors and none for overlay-manager. Baseline audit at d7cf38cb8f90712cb696043437a6982eb700f6d6 reproduces those 14 plus a stale index; regeneration removes the latter.
- The q.py baseline launch failed before Python because Bash shared-memory mapping was denied; direct PowerShell/Python baseline and current checks completed with the failures described above.
- Native interception, production Draw equivalence and remaining OV1/OV7/OV8 contracts were not tested or changed. No ABI row proposals.

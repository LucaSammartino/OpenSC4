| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gr32 — GR12 view-publication residual (docs) | Codex / re-gr32 / reserved re-gr32; detached HEAD | Ready docs; spec commit 2926a0183; doccheck pass; questions index regenerated; global questions check has 11 pre-existing errors also present in main; cargo/clippy/ctest n/a; no new game run or shadow result; GR12 runtime-narrowed; no ABI/seam changes | 1. Obtain matched original-DLL RE replay and complete publication-write trace. 2. Keep s0 acceptance (c) and replacement credit held until that observation. |

GR12 imported from replace-cfgrules30; the question file was already identical to main.
Updated graphics-rules.md §2.7 with instruction-checked publication, refresh and Shutdown guards.
Added three worked observer/provider cases and exact first-null-writer/missing-publication criteria.
Preserved original, corrected-source, unrelated fault and incomplete observation evidence separately.
Current r1 s0 on1.log:11973 is +0x1DF76E; only the named on2 log supplies +0xE2C06.
Archived GR12 with ANSWERED 2026-10-27 / DIFFERS, explicitly runtime-narrowed; regenerated QUESTIONS.md.
doccheck check and git diff --check pass; the 11 global question errors reproduce in main unchanged.
No engine/ access, native storage address in the spec, ABI rows, debugger session or game replay.
REA was not needed: exported instructions cover the helpers, including the failed refresh decompilation.
Missing: same failing source/DLL/save/script/graphics/window settings under both cfgrules settings,
with gap-free native publication writes, writer/thread/stack and contemporaneous public-view evidence.

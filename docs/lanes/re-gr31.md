| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gr31 — graphics-rules GR11; kind docs | Codex; C:/Users/tlori/FreeCity4/.claude/worktrees/re-gr31; detached HEAD, reserved branch re-gr31 | ready; spec commit 1ea2df6b1, followed by this report commit; ABI/seam reservations none; doccheck passed; cargo/clippy/ctest n/a; new game/shadow n/a; GR11 answered with runtime attribution narrowed | 1. Coordinator lands docs. 2. replace-cfgrules30 checks acquisition ownership and obtains §2.7's first-null-writer/lifecycle trace before resolving s0 acceptance (c). |

GR11 is archived in docs/specs/questions/answered/graphics-rules.md; the generated question index is refreshed.
Graphics-rules §2.4 corrects main-window acquisition to cIGZWinMgr; §2.7 specifies the native Init/evaluate order and borrowed/counted references.
No extra saved-state restore or application-specific manager IID was found; three worked interface cases distinguish view lifetime, facade use and repeated Init.
The +0xE2C06 fault is a null view reference in an audio-ambience condition before GetCurrentViewLevel; the key-off +0x3529C0 terrain dereference is distinct.
Residual observation: identify the first null writer (or missing publication), its stack, and view construction/pre-city/Shutdown plus acquisition/release history in a matched replay.
Static export/decomp analysis only, with instruction-byte checks; no REA needed. No engine files read or changed, no ABI rows proposed.
Direct `python tools/doccheck.py check`: passed. `git diff --check`: passed. `questions.py index`: completed; GR11 lookup is answered.
The q.py-wrapped doccheck attempt failed before Python because Git Bash CreateFileMapping was denied; the later direct PowerShell invocation passed.
Repository-wide `questions.py check` reports seven existing open/answered overlaps outside this task: AP19, EX58, NM18–NM20, NO46–NO47; those files were left unchanged.
Supplied evidence remains 77/77 properties, 12/12 recommendations and 13/13 owner-shadow snapshots; no new game run, shadow result or replacement credit is claimed.
The spec commit succeeded on detached HEAD despite a packed-refs.lock permission warning; no shared refs, branches or Git maintenance were requested.
Next: runtime fault attribution and s0 acceptance remain with the coordinator/engine lane; the documentation task is complete.

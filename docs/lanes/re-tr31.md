| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-tr31 terrain-view TR47; kind docs | Codex; C:/Users/tlori/FreeCity4/.claude/worktrees/re-tr31; detached HEAD, reserved branch re-tr31 | ready (docs); commits 95d976ebe, 3cfc5b2b4, 7a2b123f2; ABI exports unchanged, no ABI rows/seam reservation; doccheck passed; game/shadow n/a; TR47 answered/narrowed | 1. Obtain the native capture specified in terrain.md §15.5.15.8 before complete-frame comparison or replacement registration. |

Date: 2026-10-26.
Imported only docs/specs/questions/terrain.md from replace-terrainview29; retained the base checkout's other files.
Terrain.md §§15.5.15.5–8 add edge-index bytes, qualified UV/rounding rules, draw-time byte vectors and auxiliary/extension control streams.
Corrected water-row culling from the instructions: water maximum >= visible minimum and strictly > land minimum; equality with land minimum fails.
Worked cases cover fixed edge words, four rotations of UVs, floor/textured/next-floor alpha, 16-metre boundary conversion, height equality, plumbing restores, LevelCell prefix/index edits and extension empty/nonempty flushes.
Archived TR47 as ANSWERED/DIFFERS with an explicit narrowed runtime remainder; regenerated QUESTIONS.md (terrain: 8 open, 39 answered, next TR48).
Gate: doccheck check passed; git diff --check passed. Cargo/clippy/ctest n/a (spec text only).
Global questions check failed on 7 pre-existing duplicate open/answered IDs: AP19, EX58, NM18, NM19, NM20, NO46, NO47. Their open/archive files are unchanged from the base commit.
No native game/frame capture was run; no complete-frame parity or replacement credit is claimed.
The remaining capture contract records resource identities, cache producers, all ordered context/driver/extension calls, draw-time referenced bytes and state restores, with separate layers, edits and optional modes.
Still missing: native resource-derived land/water/edge bytes, edge-intersection rounding, full grid/plumbing/levelled-quad geometry and extension batching/remapped bytes for the recorded scenes.
Only export queries, local decompilation and executable constants were used; REA was not needed. No engine files were read or edited.
No ABI proposals, exports, game files or decompiler output are committed.

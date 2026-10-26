| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzf30 / gz-framework-core GZF13; kind docs (RE) | Codex / `.claude/worktrees/re-gzf30` / detached HEAD, reserved branch `re-gzf30` | ready; spec commit `480e324ff`; doccheck check + links passed; cargo/clippy/ctest n/a; game n/a; shadow n/a; GZF13 answered; ABI exports unchanged, no ABI/seam reservations | 1. replace-gzfw29 implements secondary slots 3–4 from §6.7 and GZB9/GZC75–GZC80. 2. Resume replacement game qualification. |

GZF13 is settled statically in `docs/specs/gz-framework-core.md` §6.7.
The secondary table at `0x00AC13EC` has five entries: QI/AddRef/Release and RGBA metadata setter/getter.
All use secondary ECX; lifetime entries adjust by -4; color entries pop 16 bytes and return AL.
The getter writes four individual bytes or leaves every output untouched for the all-255 sentinel.
Color metadata is independent of primary map/unmap, pixels, key, modification counter and cache invalidation.
`RZDrawBltEdge` calls secondary getter offset `+0x10` at `0x008D889C`, returning to the reported `0x008D889F`.
Added worked case GZB9 and six conformance descriptions, GZC75–GZC80; no engine files accessed or edited.
Imported the question file from replace-gzfw29; only GZF13 differed from the base. Archived its full text and answer; ran questions.py index.
Sources: original PE table words, export instructions with bytes, and bounded decomp ranges. REA was unnecessary.
Direct PowerShell doccheck check and links both passed. q.py could not start Bash (Win32 error 5); that wrapper attempt did not run the gate.
The spec commit completed despite a shared packed-refs.lock permission warning; detached HEAD and clean tracked status were verified afterward.
Game runs/shadow checks were not performed; replacement qualification belongs to replace-gzfw29.
No ABI proposal file; the complete calling contract is in §6.7. No C ABI export changes.
Next: land these docs, implement the two missing secondary entries, and run replacement qualification.

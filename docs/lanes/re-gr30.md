| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gr30 graphics-rules paths; kind docs | Codex / re-gr30 / detached HEAD (reserved branch re-gr30) | ready; docs merge 9b78303e3, specification 4925906e0; ABI exports unchanged, no ABI rows or seam reservations; questions check PASS, whitespace PASS; cargo/clippy/ctest n/a; game n/a, shadow n/a; GR10 answered | 1. Land docs queue. 2. replace-cfgrules28 implements the published paths and repeats rule-query comparisons. |

Answered GR10 in graphics-rules.md §§3.1 and 6.2; moved the original question to questions/answered/graphics-rules.md and regenerated the index.
Required rules: cISC4App IID 0x26CE01C0, slot 53 (+0xD4), output string and signed language -1.
Optional options: same interface, slot 58 (+0xE8), output string. Automatic log: slot 51 (+0xCC), output string.
Specified literal suffixes, file-parser class/IID/slots, direct Windows opens, order, duplicate selection, and every one/both/neither-directory case.
No alternate-directory search; missing required rules still cause fallback when options exist. Missing options alone are ignored.
Evidence: original export and decompilation; dispatch, arguments and open modes checked against instructions. REA was not needed.
Checks: questions.py check passed; git diff --check passed; engine tree equals the starting checkout after the docs-only merge.
No new game runs or parity claim. Existing cfgrules28 query differences remain implementation-side validation work.
No changes outside the requested specification, question/archive/index and this lane record; no ABI-row proposal file.

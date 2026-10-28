| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-hg32: HG9 native variant layout (kind docs) | Codex / re-hg32 / detached HEAD; reserved branch re-hg32 | ready; docs commit 1d748aae9; doccheck passed; diff check passed; game n/a; shadow n/a; HG9 answered; ABI exports unchanged; no ABI/seam reservations | 1. Land docs. 2. replace-foundations32 uses foundations §17.1.1 for its native layout and tests. |

Imported the question-only difference from replace-foundations32; no engine files were read or changed.
Published [foundations §17.1.1](../specs/foundations.md#1711-native-instance-layout-hg9-compat).
Native size 20, alignment/packing 4; payload +4, u16 reference count +0x0C, u16 type +0x0E, u32 element count +0x10.
Property variant view is owner+4; its reference word is shared with the owner's reference operations.
Constructor/setter/accessor/reference instruction widths establish the layout; no REA calls were needed.
Three worked cases cover Uint64 aliases, array storage/Erase, and embedded-property offsets/reference sharing.
Linked variant-storage §3 and archived HG9 as ANSWERED 2026-10-27 MATCHES; regenerated the question index.
`doccheck.py check`: passed. `git diff --check`: passed after removing a trailing blank line.
`questions.py find HG9`: exactly one result, answered archive; no open HG9 remains.
`questions.py check`: failed with 14 findings, all in unchanged question files outside foundations; left out of scope.
No runtime tests or game runs; worked cases are static expectations, with no verification/replacement credit claimed.
Nothing remains unresolved for HG9. No proposed ABI rows; no C ABI export change.

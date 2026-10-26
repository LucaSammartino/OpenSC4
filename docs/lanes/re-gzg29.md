| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzg29: graphics question dispositions; kind docs | Codex / C:/Users/tlori/FreeCity4/.claude/worktrees/re-gzg29 / detached HEAD, reserved re-gzg29 | ready; RE commits b5c5815f3, ace611eb4, f75145b33; ABI exports unchanged, no seam reservation; doccheck pass, questions check pass, diff check pass; game n/a, shadow n/a; GZG7 settled, GZG1/GZG2/GZG3/GZG5 narrowed | 1. Land docs. 2. Capture the native observation sets in the cited spec sections before extending compatibility credit. |

- Expanded docs/specs/gz-graphics.md with prose contracts, numeric widths, address evidence and worked cases.
- Section 3.2.6 settles GZG7: x87 Horner evaluation, binary32 stores, AA perpendicular corners, origin-zero degenerate quads, non-AA axis-dependent caps.
- Section 3.2.7 adds non-AA ellipse topology/counts and integer-line phase/tie examples; fractional diamond ties/thin clips and trig last-bit agreement have explicit captures.
- Sections 4.3–4.4 add the SimGL extension prefix, base defaults/Shutdown, fallback and image-cache failure ownership; exact device/AA output remains a stated capture requirement.
- Section 5.4 adds driver/INI precedence, folded slots 17/20, ordered UI driver calls and cached projection; concrete IO signatures/ownership, rollback and presentation remain capture requirements.
- Section 6.2.1 adds ordinary alpha lock/failure order, low-byte return, five-bit interpolation and 565 green-bit loss; specialized kernels retain explicit per-operation captures.
- Moved all five requested entries to docs/specs/questions/answered/gz-graphics.md with dated dispositions; regenerated docs/specs/QUESTIONS.md.
- No runtime capture was performed; four narrowed entries are not claims of full native compatibility or blanket owner credit.
- Gate: python tools/doccheck.py check; python tools/questions.py check; git diff --check — all passed. Cargo/clippy/ctest n/a (spec-only).
- No REA calls; failed SimGL export pseudocode was investigated through paged instruction bytes. No engine/, docs/abi/, README or HANDOFF reads/edits.
- ABI proposal file: none. In-game runs and shadow comparison: n/a.

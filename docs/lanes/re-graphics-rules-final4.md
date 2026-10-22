| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-graphics-rules-final4: GR4–GR6 final RE round | Codex / `re-graphics-rules-final4` (docs lane; detached HEAD) | in progress | Trace GR4–GR6 and publish source-backed rules. |

## Handoff
- Goal: resolve only GR4–GR6 from actual export/decomp evidence; final logical round, park anything incomplete.
- Done: initial handoff `1ff6de77`; verified GR5 x87 unordered branches, corrected V1, moved GR5 to answered.
- Done: §6.1 documents actual Options/Properties headings, 21-byte label prefix, CRLF, split payload/terminator writes, ignored write/close failures.
- Next: finish GR4 shared grammar/dependency trace and GR6 lifecycle/format failure details; add discriminating vectors.
- Facts: main `C:/Users/tlori/FreeCity4`; manifest `C:/Users/tlori/FreeCity4/build-runs/run4-re-graphics-rules-final4-r1-manifest.json`.
- Shell fallback: q.py cannot start Bash (E_ACCESSDENIED); native PowerShell local reads remain available.
- Constraints: docs/spec/questions only, no subagents, no restart, no ABI edits, no game runs or implementation evidence.

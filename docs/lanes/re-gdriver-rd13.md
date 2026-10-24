| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gdriver-rd13 (RD6/RD7/RD12 contracts) | Codex / `re-gdriver-rd13` | in progress | Resolve converter arithmetic, CopyColorBuffer and surface aliases |

## Handoff

- Goal: answer RD6, RD7 and RD12 with instruction-backed rules and conformance vectors; no engine reads/edits.
- Done: instruction-backed §12.6 CopyColorBuffer and §14.2 owner binding rules drafted; core closure rows read.
- Next: run questions index/doccheck, review arithmetic vectors and finish report/queue.
- Done: §9.6 ordinary conversion and DXT rules; DX7-50–62; RD6/RD7/RD12 archived.
- Facts: CopyColorBuffer ECX is owner+4; Flush ECX is owner+C. Probe cache is owner+288/+289, constructor-only reset.
- Surface aliases: +228 primary; +22C render target; +230 depth; +234 never populated. Loss releases first three.
- RD12 source: `git show core-gdriver-close12:docs/specs/questions/renderer-driver.md`; not yet present locally.
- Main export/decomp roots: `C:/Users/tlori/FreeCity4`; REA unavailable by task instruction.
- Detached HEAD is intentional; commit normally without moving branch refs.

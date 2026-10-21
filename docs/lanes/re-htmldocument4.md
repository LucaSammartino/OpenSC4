| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-htmldocument4 (RE spec of exact owner cHTMLDocument, 70 fns / 38220 B) | `re-htmldocument4` / (RE spec lane) | in progress | see Handoff |

## Handoff
- Goal: docs/specs/html-document.md + questions/html-document.md for exact owner cHTMLDocument (70 fns 008fe6e2..009081bb incl. 2 hashtable helpers + thunk).
- Done: worktree created.
- Next: read ctor/Init/ParseHTMLDocument/GetToken/ParseAttributeList from decomp/cHTMLDocument.c (5841 lines; offsets via `grep -n "^// ---- "`), write spec groups.
- Facts: inventory via `python tools/re_query.py search cHTMLDocument --limit 100`.

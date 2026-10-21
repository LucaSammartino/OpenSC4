| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-htmldocument4 (clean-room Rust core of cHTMLDocument per docs/specs/html-document.md) | `core-htmldocument4` / `.claude/worktrees/core-htmldocument4` | in progress | implement engine/core/src/html_document |

## Handoff
- Goal: engine/core/src/html_document (Rust only): tokenizer, attributes, lifecycle, tags, styles, entities, downloads per spec; conformance tests; full gate.
- Done: worktree at main 2b91de8c.
- Next: read spec sections, write module.

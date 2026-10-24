| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-watercraft-wc13 (WC10–WC12 behavior closure) | `re-watercraft-wc13` / RE lane, detached worktree | in progress | (1) Read owner closure rows; (2) resolve WC10–WC12; (3) doccheck and queue. |

## Handoff
Goal: close WC10, WC11 and WC12 with behavior and conformance vectors.
Done: merged core-watercraft-close12 prerequisite (merge reported success despite packed-refs lock warning).
Next: inspect owner README rows and question text, then query export evidence.
Facts: no engine edits authorized; only the explicitly requested owner README is read. REA unavailable; use main checkout export and decomp. No game runs or ABI changes planned.

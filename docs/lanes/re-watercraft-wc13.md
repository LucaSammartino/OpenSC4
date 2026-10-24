| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-watercraft-wc13 (WC10–WC12 behavior closure) | `re-watercraft-wc13` / RE lane, detached worktree | in progress | (1) Read owner closure rows; (2) resolve WC10–WC12; (3) doccheck and queue. |

## Handoff
Goal: close WC10, WC11 and WC12 with behavior and conformance vectors.
Done: prerequisite merge 0c46c082b; WC12 d978026d6. WC10/WC11 contracts and 20 vectors added in §§3.7–3.8/11.20; all three questions archived, no residual splits.
Next: regenerate questions index, doccheck in foreground, final report and queue.
Facts: no engine edits authorized; only the explicitly requested owner README is read. REA unavailable; use main checkout export and decomp. No game runs or ABI changes planned.

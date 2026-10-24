| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-effects-ef21: native observation and payload contracts | Codex / re-effects-ef21 (detached HEAD) | in progress: EF21 specified | Run doccheck and enqueue documentation. |

## Handoff

- Goal: answer EF21 from glue-effects-capture8, with spec behaviour and conformance cases; no engine edits.
- Read capture8 exact coverage boundary: no wholly compared owner; manager live/lifecycle/publication/camera and resource lifecycle/history remain omitted.
- Current main-based spec lacks capture8's §18.7 (EF20); inspect and preserve prerequisite contract before extending.
- Done: be8f43a11 preserved prerequisite §18.7; §18.8 now answers EF21 and supplies 13 unexecuted vectors; question archived.
- Verified original call sites, return widths, reload order, copy widths and nested vector strides in export instructions with bytes.
- Next: regenerate question index, run doccheck and commit final report/queue.
- Evidence: main checkout export/decomp, read-only. No REA session used, no game/build request.
- Finish: questions index, doccheck, committed report, kind re ready queue using actual detached HEAD.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netrules-shadow5: network rules record-only shadow | Codex, `glue-netrules-shadow5` (engine lane) | in progress | Audit NR16 observation prerequisites, implement if complete, validate through 2032. |

## Handoff

- Goal: default-off same-process netrules_shadow, exact through 2032.
- Done: merged requested re-netrules-nr16 at 15a6248d on detached HEAD.
- The merge includes inherited unrelated commits; no forbidden source files were opened.
- Merge reported a packed-refs.lock permission error after creating the merge commit; HEAD confirms success.
- Read network-rules section 2.5: semantic layouts and capture requirements now specified.
- Native facts are explicitly only proposed for ABI publication; docs/abi contains no tile-evaluator contract.
- Section 2.5.5 requires exact loaded observation-site byte checks, but supplies spans, not expected bytes.
- Next: finish bounded compat audit; if no approved byte check exists, file NR17 and stop wip as instructed.
- No engine edits, builds, game requests, or outstanding asynchronous operations.


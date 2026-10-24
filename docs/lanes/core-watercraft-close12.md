| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-watercraft-close12 (WC4/WC5 owner closure) | `core-watercraft-close12` / engine lane | in progress | (1) implement settled contracts; (2) run all and verify; (3) commit report and queue. |

## Handoff
Goal: close settled cSC4Watercraft WC4/WC5 contracts in engine/core/src/watercraft.
Done: added AI target arithmetic, inherited tween/animation/visibility/lighting and shared effects binding, heel/contact composition and separate crash dispatch with vectors.
Next: collect focused operation session 53091, correct diagnostics, run stable all and verify once, finalize report and queue.
Constraints: no capi/plugin/ABI changes. Partial closure: common ticks, picker orchestration, player drive, far docking and earlier owner-table pending work remain.
Spec discrepancy: WC12 filed for heel parameter/basis contradictions. Literal current completion section and explicit banking call used.
Gate: focused compile still running; build-runs/focused-1.log, same operation session 53091. Game requests: none.

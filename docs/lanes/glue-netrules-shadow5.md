| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netrules-shadow5: network rules record-only shadow | Codex, `glue-netrules-shadow5` | in progress | Audit native comparison contracts in the spec and SDK. |

## Handoff
- Goal: default-off netrules_shadow comparing loaded rules and evaluations, exact through 2032.
- Done: reserved detached worktree reused; initial tree clean; audited spec sections 2.1-2.4 and 5,
  docs/abi, SDK/plugin references, NetToolHost, NetMgrShadow, TrafficShadow and compatibility API.
- Finding: direct-only Evaluate has no published return type, concrete neighbourhood/output ABI or safe
  observation/replay contract; native tree and intersection-map traversal layouts are not specified.
- Filed NR16 in docs/specs/questions/network-rules.md; loading is lazy at original tool Init.
- Next: wait for spec/ABI answers to NR16; resume engine implementation only after those contracts exist.
- Constraints: no native signatures may be guessed; file a numbered question and stop wip if blocked.
- No checks or game requests started.

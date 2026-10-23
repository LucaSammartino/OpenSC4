| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netrules-shadow5: network rules record-only shadow | Codex, `glue-netrules-shadow5` | wip: blocked on NR16 | Publish native table/evaluation observation contracts, then implement and verify through 2032. |

## Report

The requested default-off `netrules_shadow` is blocked by missing native interoperability contracts.
Commit `4965eb6a` files NR16 in `docs/specs/questions/network-rules.md`. No engine source, C ABI exports,
configuration keys or regression profiles changed.

The audit covered network-rules sections 2.1-2.4 and 5, relevant docs/abi and SDK/plugin references,
NetToolHost, NetMgrShadow, TrafficShadow and the compatibility API. Section 2.4 expressly gives the solver
family no GZCOM identity or interception point. Its Evaluate description supplies thiscall and stack size,
but not a return type/channel, concrete neighbourhood representation, output-list layout/ownership or
safe observation/replay contract. The rule-tree and intersection-map traversal contracts are also absent.
Section 2.1 loads the rules lazily at original tool Init; city load alone does not establish readiness.
NR16 requests these facts and their documentation in docs/abi as required by CONTRIBUTING.md.

Validation: documentation-only stop; cargo, clippy, CMake and CTest not run. No plugin build or game
request was started. There is no real-city run record, clean/total line, shadow measurement or claim of
verification through 2032. Coverage files and tools were not accessed or changed. The question index is
generated; the coordinator must regenerate it using analysis-side tooling.

Next: resolve NR16 in the spec and ABI documentation, implement record-only comparisons with the supported
version check, run the required final gate and native verify queue, then request a unique-label real-city
run through 2032. Keep the shadow default-off and credit coverage only after every measured line is exact.

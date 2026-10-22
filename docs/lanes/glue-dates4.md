| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-dates4: DT2/DT3 date adapter | `glue-dates4` / engine lane | in progress | Reconcile published contracts, gate, and independently verify native dates and our adapter in game. |

## Handoff
- Goal: reconcile DT2/DT3 core/CAPI/cFC4Date and verify original-versus-Rust from identical inputs in process.
- Worktree reserved, detached HEAD; no branch/ref changes. No RE, subagents, tools/maps/default edits.
- Done: DT2 folded-byte host-input API and NUL/locale vectors; CAPI scalar/month/text bridge; DT3 core documentation reconciled.
- Baseline qbuild all still running (session 63337); started before edits. No baseline verdict yet.
- Next: route cFC4Date through bridge, use actual native string cleanup EAX, independent original-date probe via existing simulator GetSimDate/Clone; quick gate.
- Published B string cleanup entry is already supplied by compat::StlNodeAllocator; capture EAX after that call, never synthesize success.
- Executable CRT locale acquisition is not published; park precisely rather than use plugin CRT locale as evidence.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-dates4-r1-manifest.json.
- Game requests: none. At most one outstanding; unique immutable labels, wait without qwait.

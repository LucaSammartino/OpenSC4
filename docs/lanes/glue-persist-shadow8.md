| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-persist-shadow8: persistence resource shadow | Codex, glue-persist-shadow8 | in progress | Inspect documented observation contracts and implement measurable shadow. |

## Handoff
- Goal: default-off persist shadow for resource manager and buffer resources; 2032 game evidence.
- Done: 6cc4cdd6e initial handoff; a0eafa909 implemented default-off read-only cache-type projection.
- Gate: focused test 1/1; fmt clean; qbuild all Rust tests 6939 passed, clippy 0 diagnostics.
- Native initial all: qbuild-20261024-061011-build-ctest PASS build 250/250 edges, CTest 78/78.
- Diagnostic after gate: persist_shadow was absent from known-key logging; appended it to FC4Config.cpp.
- Initial gate evidence is preserved in build-runs/qbuild-*.log and fc4verify-*.log; final gate needed after config fix.
- Scope: core Filter::Type and KeyList append vs native GetResourceListForType, all key fields/order/duplicates.
- Questions: PR9 registry/cache/refcount independent replay; SCR11 buffer replay inputs. No whole-owner credit.
- Next: final all gate after known-key fix, then 2032 turbo game with persist_shadow=1.
- Constraints: clean-room; no tools reads/edits; no branch/ref changes; no agents.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-persist-shadow8-r1-manifest.json.
- Outstanding game requests: none.

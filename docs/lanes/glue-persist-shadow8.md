| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-persist-shadow8: persistence resource shadow | Codex, glue-persist-shadow8 | in progress | Inspect documented observation contracts and implement measurable shadow. |

## Handoff
- Goal: default-off persist shadow for resource manager and buffer resources; 2032 game evidence.
- Done: 6cc4cdd6e initial handoff; a0eafa909 implemented default-off read-only cache-type projection.
- Gate: focused test 1/1; fmt clean; qbuild all Rust tests 6939 passed, clippy 0 diagnostics.
- Native initial all: qbuild-20261024-061011-build-ctest PASS build 250/250 edges, CTest 78/78.
- Diagnostic after gate: persist_shadow was absent from known-key logging; appended it to FC4Config.cpp.
- Initial gate evidence is preserved in build-runs/persist-initial-*.log.
- Corrected source: d808ddd39; final all Rust 6939/6939 and clippy 0 diagnostics.
- Final all PASS: native qbuild-20261024-061927-build-ctest, build 205/205 edges, CTest 78/78.
- Scope: core Filter::Type and KeyList append vs native GetResourceListForType, all key fields/order/duplicates.
- Questions: PR9 registry/cache/refcount independent replay; SCR11 buffer replay inputs. No whole-owner credit.
- Next: collect game verdict with build-runs/wait-persist-verdict.ps1 (file-event wait, no qwait).
- Constraints: clean-room; no tools reads/edits; no branch/ref changes; no agents.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-persist-shadow8-r1-manifest.json.
- Game request: persist-shadow8-r1-2032-20261024a, turbo to 2032, persist_shadow=1, enable=simulator.
- Planned run record: docs/runs/persist-shadow8-r1-2032-20261024a-434.md (existing max 433).

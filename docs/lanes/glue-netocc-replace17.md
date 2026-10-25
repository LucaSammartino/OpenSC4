| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netocc-replace17 | Codex / glue-netocc-replace17 | ready (measured partial) | Implement occupant factories; do not append netocc to the house list. |

Partial delivery: no `netocc` replace key or occupant factories were added. The credit flip remains unfinished.
Core commit d7d47e4b8 hydrates live PowerLine state from v2 records and snapshots it back; saved cells, float bits and pole identities survive, while preview remains transient.
The default-off netocc shadow now exercises that live-state mapping and reports nonempty per-class sample totals.
The plugin factory-isolation CTest checks that netocc_shadow alone registers none of the thirteen occupant classes.
Gate: qbuild all PASS; Rust 7526 passed, 0 failed; clippy 0 diagnostics; native build 274/274; CTest 85/85, including the new isolation test.
Native queue record: qbuild-20261025-013722-build-ctest; logs build-runs/fc4verify-build.log and build-runs/fc4verify-ctest.log.
Exact tally: `netocc-live-r17-20261025a 73/73`; every netocc sample has mismatches=0.
Run: shadow-only Konradshohe, turbo, clean at 2032-01-09; ref_warnings=0, exceptions=0, panics=0.
Run record: build-runs/game-results/netocc-live-r17-20261025a.verdict; plugin evidence in the corresponding .log.
Sampled records: network 62561, pipe 29419, pipe connection 146, power pole 219, power line 1314; each class exact 73/73.
The other eight classes had zero samples and are not claimed as measured.
Questions: none filed; SPEC-GAP comment identifies incomplete inherited folded/lazy-placeable behavior in occupant-manager section 13.1.
Missing: the inherited occupant adapter, netocc key, all requested factories, and both replacement runs to 2032.
Next: implement the factory adapters, then run netocc-only and full-house replacement trials. This result does not authorize changing the house replace list.

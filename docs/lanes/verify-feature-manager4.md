| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-feature-manager4: independent native query verification | `Codex` / `verify-feature-manager4` (engine lane) | in progress | Audit FM3/FM4; implement independent native query/lifetime verification; gate and game sessions. |

## Handoff
- Goal: compare app-owned native cSC4FeatureManager query behavior with independent Rust inputs under published contracts.
- Base: detached HEAD 17319e21; reuse reserved worktree, never update refs.
- Done: ae70c86f adds independent snapshot C ABI, app-owned native query/lifetime verifier, exact nine-slot support-image validation. Code committed before call 30.
- Baseline all: cargo 4947/4947 and clippy clean; CMake 208/208; ctest 67/68 (cargo_test compiled concurrently edited tree); baseline not an immutable gate.
- Audit: FM4 wrapping/signed comparison already correct; extend all three bit vectors. FM3 complete-byte provider cannot inject streaming faults; incomplete captures explicitly refused.
- Focused quick: PASS 3451 tests, dependents compile, CMake160/160; changed-tree diagnostic means final immutable gate still required.
- Format: touched feature core/tests and CAPI module pass qbuild fmt (absolute paths required); lib.rs recursive check also visits pre-existing unrelated format differences (no mass reformat).
- Next: full immutable qbuild all; freeze DLL, record hash and submit unique guarded2032/ref0 query and lifetime sessions; wait without qwait.
- Restrictions: no native Init/Shutdown/reset/seeding; borrowed app getter and owned QI hold only; FM1/FM2 excluded.
- Game requests: none outstanding. Unique labels required; wait without qwait.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-verify-feature-manager4-r1-manifest.json.

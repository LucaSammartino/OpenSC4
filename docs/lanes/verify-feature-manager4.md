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
- Added independent Rust lifetime mirror (fixed published app AddRef + verifier QI lifecycle, never seeded from native counts); native supported/unsupported QI and AddRef/Release compare against it.
- Final unchanged-code all at 0465e0bf: PASS cargo4948/4948, clippy0 diagnostics, CMake139/139, ctest68/68. Earlier in-flight all also passed but is not final evidence.
- Frozen DLL: build-runs/dlls/FreeCity4-feature-0465e0bf.dll; SHA256 BD01E7356043EB756C2FDB895FF4E0CB4533013AB7DAED1EBC62943D52F7D777.
- Query fm4-r1-query-1022-171537: clean2032/ref0; samples74, exact13172, diff0, refusal0, resync0; lifetime15 exact/0 diff; native AddRef3/Release2 = Rust3/2; final app hold1. All32 provider keys captured; only pack1 acquired (378B/FNV BC8924D4).
- FM5 filed in docs/specs/questions/feature-manager.md: actual provider size378 versus §5.1 measurement263; no input normalization or native-answer seeding. Shadow differences remain0.
- Outstanding: none. Next: lifetime session on same frozen DLL with scripted full/fast saves via feature-submit.ps1 lifetime; accept its unique completed verdict, then final report/queue.
- Restrictions: no native Init/Shutdown/reset/seeding; borrowed app getter and owned QI hold only; FM1/FM2 excluded.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-verify-feature-manager4-r1-manifest.json.

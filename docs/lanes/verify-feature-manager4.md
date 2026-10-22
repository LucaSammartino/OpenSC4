| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-feature-manager4: independent native query/lifetime verification | `Codex` / `verify-feature-manager4` (engine lane; detached reserved worktree) | ready; cargo4948/clippyPASS/ctest68; 2 clean2032/ref0 sessions; queries26344 exact/0 diff/0 refusal/0 resync; lifetime30 exact/0 diff; FM5 | 1. Merge bounded verification. 2. Reconcile FM5 provider-size measurement; FM1/FM2 reachability stays outside this evidence. |

## Report

Implemented opt-in `feature_shadow=1`, comparing the app-owned original with
`engine/core/src/feature_manager` through a new snapshot C ABI. The public SDK
app slot32 is borrowed; IID0B562673 supplies the verifier's owned hold. All nine
original slots, the exact vtable, supported image, readable object and safe live
count are validated before native use. No original Init, Shutdown, constructor,
destructor, state writes, static resets, factory replacement or Release-to-zero.

The public resource-manager raw-record / serial / file-adapter path captures all
32 actual keys once per process. Rust parses the captured bytes once, retaining
every mentioned feature ID, including disabled IDs. Native answers never seed
Rust state. Failed/incomplete/oversize captures refuse comparison instead of
becoming absent packs. Seventeen fixed feature edges plus captured IDs are
queried, together with all32 pack residues at five u32 boundaries and current
pack, at load, each month, and before city shutdown. Resync is never performed.

The separate Rust lifetime model follows the published app AddRef plus verifier
QI lifecycle; it never accepts a native reference count as input. Native AddRef,
Release, both supported QIs, failed-QI output preservation and final release of
our owned hold are compared. Each run observes native/Rust AddRef3 and Release2,
then final app_hold_remaining1. Borrowed pointers are never released.

FM4 wrapping/signed-release logic was already correct; its tests now cover all
three published independent bit vectors, retained static state, initialized-byte
clearing and destruction/no-destruction outcomes. The C ABI test covers copied
inputs, absent/empty packs, numeric keys, off values, pack wrap, current pack,
mentioned IDs, lifetime mirroring, invalid/null arguments and capture refusal.
Four additive C ABI exports; ABI version remains84. No ABI tables/defaults/maps
changed. Code milestones: ae70c86f, f46d0c5a, 8045b3c1, 0465e0bf.

Final unchanged-code `qbuild all` at 0465e0bf: cargo4948/4948, clippy0 diagnostics,
CMake139/139, ctest68/68 PASS. Focused quick also passed3451 tests/dependents and
160 CMake edges before the final lifetime extension. Initial baseline all ran:
cargo4947/clippyPASS/build208, but ctest67/68 overlapped edits; that run and the
subsequent changed-tree gate are not final evidence. `qbuild fmt` passes the
feature module/tests normally and all four touched Rust files with
`skip_children=true,reorder_modules=false`, avoiding pre-existing recursive
format and module-order changes in additive shared lib.rs. `git diff --check`
passes. Full logs remain in build-runs/qbuild-*.log and fc4verify-*.log.

Both sessions used the same immutable DLL:
`build-runs/dlls/FreeCity4-feature-0465e0bf.dll`, SHA256
`BD01E7356043EB756C2FDB895FF4E0CB4533013AB7DAED1EBC62943D52F7D777`.
Current40 profile exactly as assigned, budget switches enabled, refaudit1,
turbo to2032-01-01; zero exceptions/stalls/RTC/overruns/reference warnings.

| Run | Verdict | Query counters | Lifetime counters | Additional evidence |
|---|---|---|---|---|
| fm4-r1-query-1022-171537 | clean2032/ref0, 114.125s | samples74; exact13172; diff0; refusal0; resync0 | exact15; diff0; app hold1 | 32 input keys; pack1 present, others absent |
| fm4-r1-lifetime-1022-171917 | clean2032/ref0, 172.359s | samples74; exact13172; diff0; refusal0; resync0 | exact15; diff0; app hold1 | Full save2028 ok1/13062ms; fast save2030 ok1/516ms |

Totals: 148 samples; queries26344/26344 exact; lifetime30/30 exact. Both sessions
capture pack1 as378 bytes/FNV-1a BC8924D4. **FM5** in
`docs/specs/questions/feature-manager.md` records the discrepancy with §5.1's
263-byte measurement. Actual provider bytes were used without normalization.
Question index regeneration belongs to the coordinator; tools/ is prohibited.

Evidence is bounded to actual inputs and these pure query/owned-hold operations.
No native positive-feature fixture, alternate installed-pack state, streaming
failure injection, overflow/destruction injection, Init/Shutdown/static teardown,
or FM1/FM2 reachability claim. FM3's complete-byte-provider limit is explicit;
FM4 boundary destruction is core conformance evidence only. No whole-owner
replacement, default enablement or coverage promotion is inferred.

No requests outstanding. Merge the bounded verifier; retain FM5 and existing
FM1/FM2 questions. Manifest:
`C:/Users/tlori/FreeCity4/build-runs/run4-verify-feature-manager4-r1-manifest.json`.

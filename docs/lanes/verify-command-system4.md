| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-command-system4: native command verification | Codex / `verify-command-system4` (engine lane, detached HEAD) | in progress; binding feasibility audit before baseline | 1. Confirm published native acquisition, variant storage and lifetime within 15 calls. 2. Implement only if fully supported. |

## Handoff
- Goal: compare production command core methods with independently owned ORIGINAL parameter sets and documented public generator operations.
- Done: confirmed public service/IID (section 5, GZServPtrs/cRZSysServPtr), slot 5 owned factory, parameter slots 3-14 and SDK typed variant calls before baseline; no layout-dependent native storage needed (scratch ORIGINAL variant).
- Done: added opt-in commands_verify load probe and private persistent production-core transport. No public C ABI/export map/default changes.
- Milestones: implementation `39531ad2`; baseline full PASS (4961 Rust tests, clippy 0 diagnostics, ctest 68/68). Focused quick PASS (530 capi tests, C++ 5/5 build edges).
- Focused new Rust fmt PASS. Full candidate gate: Rust 4961 PASS, clippy clean, C++ build PASS; ctest 67/68 because return-channel audit required factory-result discard, published parser AL citation and removal of a local Create-name collision. Corrected without maps/table edits.
- Stable full gate PASS at source `f844935e`: Rust 4961/4961, clippy 0 diagnostics, C++ build 4/4 edges, ctest 68/68 (96.0 s). No subsequent source edits.
- Native run `commands4-native-r1-e50c5bbc-a`: 7414 assertions / 7243 exact / 171 mismatches / 0 refusals; first twelve differences scalar variant GetCount 0 native / 1 core. GC8 filed; production semantics unchanged. Original vtables in refaudit: service game+0x6D17A0, sets game+0x6D16C8. Probe took ~2 ms.
- Game reached 2032-01-01, ref_warnings=0, full save succeeded (10860 ms), full shutdown reached. Harness verdict stalled because save caused watchdog_stalls=1; do not call clean. Completed verdict retained.
- Next: aggregate mismatch-category instrumentation, stable gate again, frozen label b native/default/reload with an earlier full save to keep the save workload bounded. No runtime fixture retained from stalled run.
- Canonical current40 profile is not in the permitted named files; do not mislabel the two default-on keys as current40.
- Formatting caveat: wrapper requires absolute file paths; checking lib.rs recursively reports pre-existing child-module formatting. No mass reformat performed.
- No game request outstanding. Core receives only raw literals, never native computed values; no registrations or executions.
- GC1-GC7 remain outside measured domain; never seed core from native computations.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-verify-command-system4-r1-manifest.json`.

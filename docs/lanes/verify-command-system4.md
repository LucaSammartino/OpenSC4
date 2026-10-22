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
- Next: frozen DLL native/production-core paired run `commands4-native-r1-e50c5bbc-a`, then own/default and supported fixture reload, all to 2032 with scripted full save and shutdown. Wait via local script, never qwait; preserve completed unique verdicts.
- Canonical current40 profile is not in the permitted named files; do not mislabel the two default-on keys as current40.
- Formatting caveat: wrapper requires absolute file paths; checking lib.rs recursively reports pre-existing child-module formatting. No mass reformat performed.
- No game request outstanding. Core receives only raw literals, never native computed values; no registrations or executions.
- GC1-GC7 remain outside measured domain; never seed core from native computations.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-verify-command-system4-r1-manifest.json`.

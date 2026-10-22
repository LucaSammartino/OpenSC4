| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-span8-native4: owned span8 native adapter | `glue-span8-native4` (engine lane, detached HEAD) | ready: cargo4952/0, clippy0, ctest68/68; own/current40+span8 clean2032/ref0; each direct160/160 and native160/160 exact+changed; shipped0; one C export, no ABI version bump | Merge bounded opt-in bridge; preserve SR8-2/3/4 and no default promotion. |

## Delivered

Base `1e4c8d55` contains the reviewed/applied 40 SR9 native ABI rows. Initial code is `8a5a3d73`; complete kernel fixtures/counters are `45d931a9`. No analysis, executable/proof-script or forbidden tree reads, ABI-table/map edits, default promotion, shared ref updates, merges or pushes.

The owned SR10 C API adds `fc4_span8_render` and explicit `FC4Span8Image`/`FC4Span8Job` records. It copies metadata, checks bounded destination/source regions through the existing span8 core, rejects source/destination aliasing and refuses unsupported row13 with status3. The transaction test checks discriminating alpha-plane output, unchanged guards, late-span rejection without partial output, unsupported row13 and overlap refusal. One export added; `FC4_ABI_VERSION` unchanged.

Opt-in `enable=span8` installs typed thiscall forwarders for native slots1–5 (16/16/20/16/24-byte cleanup, void returns). The identified build and six original vtable entries are checked first. Slot0 remains the original deleting destructor. Native slot22 is void/pop8. Shadow mode (`span8_shadow=1`) retains original output, compares an independently initialized Rust arena and never resynchronizes pixels. Refusals fall back to the original with separate counters. State counters are atomic; nested original alpha/gradient redispatch is excluded from logical-call counts.

## Native evidence

Both fixture classes execute original and Rust from the same initial full arena, including row padding and unused boundary rows. Direct calls and the native factory/Init/public-context-setter/slot22 route each execute four untextured cases plus all156 supported format/key/kind combinations. Every case changes the original output. Native texture key state is constructed from the exact buffer fields in §10.7, restored after the fixture; no unreviewed native key-setter ABI is invented. Public setters configure context state. Texture and target lock counts/flags return to baseline; Shutdown clears the renderer, and context/buffer releases all return0. Per-class, per-slot and per-kernel calls/ours/exact/diff/refusal/fallback are separate, with resync0.

| Unique label | Profile | Verdict | Direct | Native | Shipped/other |
|---|---|---|---|---|---|
| s8n4-own-45d931a9-20261022a | clock24,span8; shadow1 | clean2032-01-15, ref0, 27.922s | 160/160 exact+changed | 160/160 exact+changed | 0/0; no credit |
| s8n4-c40-45d931a9-20261022b | user's current40+span8; replacement | clean2032-01-01, ref0, 108.234s | 160/160 exact+changed | 160/160 exact+changed | 0/0; no credit |

Each trace has 156/156 unique supported kernel entries with calls1/ours1/exact1/diff0/refusals0/fallback0 for each constructed class. Guards/padding corruption, fixture faults, no-call, foreign exceptions, watchdog stalls, RTC and overruns are0. All eight buffers and the context release to0. Immutable DLL: `build-runs/dll/span8-45d931a9/FreeCity4.dll`, SHA256 `B02D0A03D8CEB8CC832D2D5360A754A65B0B198964644293704AD1D2F78F6CBB`. Detailed logs/verdicts are `build-runs/game-results/<label>.*`; run rows are `docs/runs/<label>.md`. Both requests completed; none outstanding. No qwait used.

## Gates and boundaries

Baseline all passed. Focused quick passed531 CAPI tests and plugin compilation. Final stable `qbuild all` on `ea7f2e3f` passed4952 cargo tests, clippy with0 diagnostics and full native ctest68/68 (72.5s, no tree-changed warning). Final commits after this gate contain documentation only. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. `git diff --check` passed. New `span_render8.rs` passes wrapper rustfmt; checking touched `lib.rs` recursively reports inherited drift in untouched `advisors.rs` and other modules, which was left unchanged as required.

No new spec questions. SR8-2 shipped reachability remains unproved (zero shipped observations earn no credit); SR8-3 row13 remains explicitly unsupported (safety refusal measured, never compared against native); SR8-4 antialiased context bypass remains outside this bridge. No whole-owner or default replacement promotion. Native records outside the owned positive-stride/nonalias bounds conservatively retain the original path; the C adapter also supports bounded signed strides through the core.

Next: coordinator may merge this bounded opt-in implementation. Further reachability or antialiased-context coverage requires answers/evidence under the existing questions. Supervisor publishes detached commits to the reserved branch; this lane did not modify shared refs.

## Coordinator measured-shadow audit (2026-10-22)

span8_source_own_direct 160/160; span8_source_own_native 160/160; span8_source_current40_direct 160/160; span8_source_current40_native 160/160; span8_joint98_direct 160/160; span8_joint98_native 160/160.

All6 measured sets compare actual original output against production Rust/CAPI
from independent equal inputs; each case changes output,156/156 positive kernel
pairs per class, zero differences/guard faults/refusals/fallback/resync.
Final joint98 DLLbfbdd88b, real68gate/regressPASS, exact house40+span8, fullsave
and shutdown clean2032/ref0; original slot0 retained and hooked entries restored.
Source final DLLb02d0a03. Joint/source raw log hashes: build-runs/run4-train98-span8-audit.json.
This qualifies measured same-process shadow under COVERAGE.md. Natural/shipped
calls remain0: no default/replaced promotion, no SR8-2/3/4 closure or exhaustive
whole-owner claim; unsupported row13 refusal is not a native comparison.

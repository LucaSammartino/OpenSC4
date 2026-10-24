| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-overlays-shadow5: OV12 terrain overlay observation | Codex; glue-overlays-shadow5 (detached HEAD; supervisor publishes lane branch) | ready for measured subset: cargo 6854/0, clippy clean, CTest 71/71; run 351 clean through 2032, getter 73/73 and empty-state observation 292/292 exact; OV13/OV14 filed | Coordinator: append overlays_shadow to existing regress shadows string; regenerate question index; capture creation inputs and exercise live records before full-owner coverage credit |

## Result and implementation

Default-off `overlays_shadow=1` now walks all three original arenas under OV12, including exact FIFO free queues, ascending live IDs, placement handles and ordered eight-part groups. It copies every named overlay field (full flags, draw byte, stamp, bounds, primary/secondary matrices, alpha, colour, binding/source tokens), all placement/aspect/target words, and all group parts. Static cells are copied in row order with stored encounter order and duplicates preserved. Binding observations include sample-local sharing, frame counts, keyed/unkeyed group/instance identity, levels, dimensions, byte accounting and format metadata; sources expose sharing and implementation tokens without invoking unknown implementations.

The native adapter bounds pointer ranges, checks aligned/divisible extents, validates every queue and graph reference before calling getters, rejects cycles/out-of-range links, and rechecks arena headers/data, cell heads and nodes before accepting a sample. It performs no native writes, AddRef/Release, allocation of native records, Draw, collection, hooks, or replacement. Native pointers exist only in event-local observer copies and become local tokens in the Rust transport. Native getters are called only for safe live direct/indirect IDs, using the documented colour and self-lighting signatures. Existing sentinel probes remain.

The portable diagnostic module is separate from production `OverlayManager`: it checks liveness, handle/group resolution, getter/record fields, placement no-write/direct versus indirect behavior, cell membership/completeness and available binding cross-fields. It retains every named record word and ordered cell list in a semantic observation digest. It never seeds native matrices, rectangles, order or resource hashes into the production owner. The private static-library transport changes no public C ABI export or version.

Implementation milestones: `f2bdcff61` (observer), `dd3e4c626` (dynamic inactive-grid fix). Merges: `e30d7b66a` (re-overlays-ov12), initial main merge, then `506ccfe2c` (latest main before final gate). Both sides' CMake/config/lib.rs additions were retained. The refreshed main merge conflicted only in generated QUESTIONS.md totals; main's generated row was retained.

## Measured game result

[Final run 351](../runs/overlays-shadow5-r2-2032b-372.md), label `overlays-shadow5-r2-2032b`: Konradshohe, turbo, clean through **2032-01-02**, **ref_warnings=0**, all harness problem counts zero. DLL SHA256 prefix `055c5720`; 60.437 game-run seconds, 74 seconds harness wall, slot 1. Load plus 72 months sampled all four managers. Exact summary lines:

```text
overlays records shadow: 292/292 lines exact mismatches=0 incomplete=0 checks=0 overlays=0 handles=0 groups=0 cells=2392064 entries=0 bindings=0 sources=0 unknown_bindings=0 observed_words=2398196 image_bytes=0 data=unavailable replay=unavailable(OV12) scope=observation-invariants
overlays contract shadow: 73/73 lines exact mismatches=0 fields=7300 scope=sentinels
```

**This is READY for measured sentinel getters and empty-state observation only.** All sampled arenas were empty. The observer read 2,392,064 empty static lists and 2,398,196 transport words; no live records, cell entries, binding metadata or source objects were exercised in the real city. Thus the record/getter comparison count is honestly zero, even though every complete empty-state observation line is exact. Do not credit full overlay-owner arithmetic, live-record/cell-order or texture verification from this run.

[Diagnostic run 349](../runs/overlays-shadow5-r2-2032a-371.md), label `overlays-shadow5-r2-2032a`: clean through 2032-01-09, ref_warnings=0; getter 73/73 exact, records 146/292 exact with 146 incomplete dynamic samples. An observer constraint incorrectly treated dynamic shift/dimension words as an applicable static grid. OV12-I explicitly marks that index N/A; the fix accepts inactive words without using them and adds a poisoned-header regression. OV14 records the unspecified inactive values. The rejection occurred at load and every month, rather than constituting a turbo-only numerical difference. Both unique verdicts and full logs remain under `build-runs/game-results/`; the failed observation evidence was preserved. No request or wait remains outstanding. Both waits used the existing event-based PowerShell waiter, without qwait.

## Verification

Final stable engine source passed `qbuild.py --wt <WT> all`: **6854 passed, 0 failed, 0 ignored**; **Clippy 0 diagnostics** with warnings denied; native build **52/52 edges** (94.6 s); full **CTest 71/71** (146.0 s); native gate total 242.9 s. Queue `qbuild-20261024-014252-build-ctest` was collected to terminal PASS. The tree-changed annotation reflects committed lane documentation only: `git diff 506ccfe2c HEAD -- engine` is empty. Source inputs were merged before this check began; only documentation conflict completion/commits occurred during it.

The earlier stable implementation also passed all: cargo 6818/0, clippy 0 diagnostics, native 212/212 edges and CTest 71/71, queue `qbuild-20261024-012924-build-ctest`. The game diagnostic then justified the source correction and repeated full gate. Logs remain in `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log` and `fc4verify-ctest.log`; both native queue verdicts retain their summaries.

Focused portable tests cover OV12 A-L and corrupt/truncated data: **13/13 PASS** after the dynamic-header regression. The first J fixture had an index typo (12/13), corrected before the first full gate; no engine mismatch was suppressed. Private C API tests also check truncated/null buffers and untouched failure output. Native CTest exercises the actual adapter's FIFO zero links, changed data, pointer/extents safeguards, ordered lists, cycles and out-of-range IDs. Explicit owned-file formatting passed. The shell wrapper's initial bootstrap failure occurred before Cargo; focused tests used direct PowerShell Cargo afterward.

## Coverage limits, questions and landing work

OV12 explicitly states that a snapshot has no inverse to the original call stream. Independent replay of creation/removal/failed-call history, overwritten transforms, model projection and static list history is **unavailable**, not silently excluded as nondeterministic. `OV13` in `docs/specs/questions/overlay-manager.md` asks for the pre-allocation capture boundary and complete caller/service/resource data. `OV14` in the same file asks for any guarantees on inactive dynamic static-index headers; the implemented literal reading treats them as N/A.

Semantic exclusions follow OV12: byte +09 and other documented padding, allocator/object/vtable addresses, reference counts and driver-handle values. Sharing/implementation identities use observer-local tokens. Stamps and both matrices are observed without resetting them, but are not independently recomputed. Arbitrary source dimensions/pixels and current GPU image bytes are unavailable under §2.7; no driver handle is treated as pixels. The digest is observation, not an independent replay verdict. No additional nondeterministic fields were excluded.

The key remains default-off. `tools/` was not accessed under the explicit clean-room prohibition. The coordinator must append `overlays_shadow` to the SINGLE existing `shadows = ...` string in `tools/regress.ps1` (no second shadows line), and regenerate the question index. Coverage maps were not edited. Full verification still needs captured original inputs and a real-city sample containing live overlays/resources; the measured subset is ready to land now.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-overlays-flags7: ordinary decal flags | Codex; detached HEAD in glue-overlays-flags7 | ready: gate passed; both 2032 runs exact | land; coordinator may append overlays_fixture to house shadows profile |

## Report

Implemented answered OV15 in the shared ordinary-decal creation path: flags start at `0x91`, including snapshot bit 7. Binding, source and resolved-key adds inherit that rule. The existing native fixture compares all 35 fields without masks; its suffix now ends at the native full flags word `0xD1`. No plugin configuration defaults or C ABI exports changed.

Added full-word OV15-A/B/D tests covering every completed setter call, static/dynamic land/water, quality 0/5, distinct 64x64 and 128x64 bindings, false self-lighting and repeat changes. OV15-C tests cover source/key creation and null-binding nonallocation. Updated the C ABI fixture assertion. Implementation commit: `c9b87073d`.

Validation: focused terrain_overlays **64 passed**; explicit owned-file fmt **PASS**. Final-source `qbuild all` **PASS**: workspace Rust **6919 passed, 0 failed, 0 ignored**, clippy **0 diagnostics**, native build **240/240** edges (93.4s), full CTest **74/74** (143.7s), native total240.7s. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. The same all operation delegated build/CTest through coordinator queue `qbuild-20261024-043910-build-ctest`; session76896 reached terminal. No direct MSVC configure or duplicate native verify was launched. The tree-change notice reflects documentation-only commit1210a7f5a; `git diff c9b87073d -- engine` is empty. The initial q.py wrapper failed before testing due to Bash CreateFileMapping access denial; evidence remains in `build-runs/q/20261024-043301-5976.log`; direct cargo succeeded.

Both game runs used DLL SHA256 `2ffcbac8e0fa35f904f8f84b157d0521b71b1a2bf29912f62ed5c819065bab7e`, Konradshohe, turbo, target2032-01-01, observe/digest off. Both verdicts **clean**, `ref_warnings=0`, all harness problem counts0; only two expected x87 startup warnings. Every counted dated shadow line has zero mismatches and zero resyncs; no incomplete captures. Exact requested overlays lines:

```text
ovflags7-fixture-2032-20261024a 73/73
ovflags7-ro-2032-20261024b 73/73
```

| Run record | Mode / final date | Sentinel comparison | Record observation | Controlled suffix replay |
|---|---|---|---|---|
| [410](../runs/ovflags7-fixture-2032-20261024a.md) | fixture /2032-01-01 |73/73;7300/7300 fields |292/292;12264/12264 checks |292/292;10220/10220 fields |
| [411](../runs/ovflags7-ro-2032-20261024b.md) | default read-only /2032-01-06 |73/73;7300/7300 fields |292/292;empty arenas,0 live checks |0 lines;fixture omitted |

**Coverage owner rows:** the fixture supplies measured verification for the `cSTEOverlayManager` row, specifically ordinary binding add, placement/move, alpha/RGB/draw-state/self-lighting suffix and indirect handle/overlay state across its four managers. Record observation additionally checks live storage/index/resource invariants. It does **not** verify the separate `cSTEVertBufExtn`, `SC4MarkedCellView`, `SC4PathInfoView`, `SC4TrainJunctionView`, `cSC4PathView`, or `cSC4PathView::PathInfo` owner rows named in the generic brief. It does not establish full rendering, groups/model-shadow paths, original Init/history reconstruction, or image bytes. OV13 remains the documented boundary; no new original-behaviour question was needed. Tools coverage files were neither opened nor edited; coordinator should apply credit only to the measured owner/scope.

**House profile:** yes, this measured fixture is safe to include as an explicit mutation test in the house shadows profile's isolated/restored city. It adds temporary overlays and removes them through native interfaces; it is not read-only. The clean 2032 run and zero reference warnings support this use. Preserve ordinary default-off behaviour and the separate read-only mode. The coordinator must append `overlays_fixture` to the current single `shadows = ...` string in `tools/regress.ps1` after integration: this lane's strict prohibition on tools/ access and detached-HEAD/no-main-merge instructions prevent that shared profile edit here. No second shadows assignment should be added.

No normal-speed rerun was necessary: neither turbo run differed. Completed unique verdicts and logs remain under `build-runs/game-results/`; waiters94521 and66460 both reached terminal. No operation/request outstanding. No questions filed; no ABI change. Ready for integration and coordinator coverage/profile review. Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-glue-overlays-flags7-r1-manifest.json`.

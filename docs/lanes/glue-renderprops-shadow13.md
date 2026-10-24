| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-renderprops-shadow13 (render properties registry stability) | `glue-renderprops-shadow13` / engine lane, detached HEAD | ready for measured partial closure: source `5001139a2`; ABI unchanged; cargo 7421/0/0, clippy 0 diagnostics, native build 264/264, ctest 81/81; run 633 clean to 2032, ref_warnings=0, renderprops 73/73; no new questions | (1) Coordinator review profile registration; (2) provide RP7 observer/view bridge for live-state closure; (3) resolve existing RP8. |

## Changes

The base already contained default-off `renderprops_shadow`, the core transport, and isolated native value/setter/lifetime fixtures. Commit `5001139a2` shares the read-only registry comparator between the borrowed application owner and the owned fixtures. Nine owned-instance registry samples check fresh A/B, both after each of three setter passes, and surviving B after A's nonfree destruction. This adds 4,878 independently predicted comparisons per dated sample, covering name/count/lookup stability across mutation and storage independence across termination. Both owners' expected registries come from the core, never native getter results. The existing 462 finite setter operations and all-value comparisons remain intact.

The ini key remains default-off. No C ABI exports changed. Config documentation and the `finite-v2` fixture log describe the expanded measurement.

## Validation

One stable-source `qbuild all` completed with PASS: Cargo **7,421 passed / 0 failed / 0 ignored** across 51 summaries; clippy **0 diagnostics**; native build **264/264 edges**; CTest **81/81**. The native portion used coordinator queue `qbuild-20261024-204415-build-ctest`, not a local MSVC invocation. Logs remain in `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, and `fc4verify-ctest.log`. Sessions 43351 and 65544 reached terminal results; no outstanding checks or game requests.

[Run 633](../runs/grp13-2032-20261024-r1-633.md): `grp13-2032-20261024-r1`, Konradshohe, turbo, 2026-01-27 through **2032-01-01**, **clean**, `ref_warnings=0`, `foreign_exceptions=0`, `watchdog_stalls=0`, `rtc=0`, `overruns=0`; 116.719 real seconds. Exact result:

```text
renderprops 73/73
```

Every dated sample reports `fields=41940 mismatches=0`; all 73 owned fixtures report `available=1 fields=41398 mismatches=0 plan=finite-v2 setters=462 registry_samples=9`. Total comparisons: **3,061,620**, including **356,094** added registry checks. DLL hash recorded by the harness: `594bd3be`. Evidence: `build-runs/game-results/grp13-2032-20261024-r1.log` and `.verdict`.

## Scope and next steps

This is ready for the measured registry and owned-fixture scope. Live configured settings, configuration/hardware/preferences replay, and the private scroll snapshot remain explicitly unavailable under RP7. The spec requires a complete independently observed input/copy history and a lifetime-safe complete-view binding; the current late attach supplies neither. No native output was copied into expected state, no existing debug-register observer was disturbed, and no whole-owner verified claim is made. Non-C locale profiles, exceptional x87 inputs, invalid native indices, and freeing ownership operations remain outside this run's domain.

No new questions or differences were filed. Existing RP8 in `docs/specs/questions/render-properties.md` remains open for the inherited isolated constructor ABI; RP7's specified observer/view facility remains unavailable. Future live closure needs that separately coordinated facility.

The clean-room restriction forbids reading/editing `tools/`; the coordinator should inspect the current single shadows profile and register `renderprops_shadow` if appropriate after reviewing this exact partial measurement. Coverage maps and profiles were untouched. This detached worktree's commits await supervisor publication; no refs were changed and no merge was attempted.

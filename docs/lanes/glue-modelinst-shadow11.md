| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-modelinst-shadow11 (finite static fixture) | `Codex` / `glue-modelinst-shadow11` (engine lane) | WIP: final cargo 7141/0, clippy PASS, CTest 81/81; run 498 harness no-load; retry run 499 stalled beyond timeout plus 15 min, exact/total unavailable; MI14 open; public ABI unchanged | (1) Repair/collect existing modelinst-static-2032-20261024-b queue result. (2) Record measured exact/total and queue ready when clean. (3) Resolve MI14 before live or whole-owner coverage. |

## Change and scope

Source commit `d230f041b` adds default-off `modelinst_shadow`, private static-link C transport, and observer callbacks at city load, monthly boundaries and shutdown. The transport uses the existing Rust model-instances core with independent inputs. Two isolated static instances exercise default identity and a finite negative-scale/rotated/translated transform. Original live city instances are never enumerated or mutated. There are no resync operations. No public C ABI export or ABI version changes.

Each monthly line identifies `scope=isolated-static-transform-bounds` and `live=unavailable(MI14)`. This is finite fixture coverage only; it does not satisfy the requested live sampling or justify whole-owner coverage credit. The feature remains opt-in. No shared regress profile or coverage tables were edited.

## Per-owner comparison boundary

| Owner | Bodies exercised for comparison | Uncompared |
|---|---|---|
| cS3DModelInstance | Constructor defaults; GetTransform and fixture-only SetTransform; GetBounds and GetWorldBounds; GetProperties, GetOwner, GetCustomData and GetResModel default results. Transform and world bounds use independent core expectations; other defaults use published constants. Destructor body is cleanup, not a compared result. | Live instances; model/material binding; nondefault properties/owner/custom data; draw attributes, draw, pick, sort, mesh/container and reference-count behavior. |
| cS3DModelInstanceBasic | None as a native basic owner. | All owner bodies, including its distinct reference-count semantics. |
| cS3DModelInstanceAnimated | None. | All animation state, playback, frame/model binding and other owner bodies. |
| cS3DModelProcessor | None. | All processing, lighting, mesh outputs and other owner bodies. |
| cS3DResSpriteAnim and factory | None. | Resource reading, lookup, normalization and other owner bodies. |
| cS3DTransform | Defined record bytes/copy counter through static GetTransform/SetTransform; finite full-matrix uniform-scale world bounds through static GetWorldBounds. | Standalone native transform methods, dispatch variants outside the two fixtures, exceptional numeric inputs and live transforms. |

MI14 in `docs/specs/questions/model-instances.md` requests safe live enumeration, ownership, getter bindings, cache noninterference and constructor/destructor binding confirmation. The receiver-only lifetime binding is explicitly marked as the literal interpretation; unknown native getter signatures were not invented.

## Gates

- Focused transport/layout fixture test: 1/1 PASS. Owned Rust formatting: PASS.
- Stable-source `qbuild.py --wt <WT> all`: terminal PASS; cargo 7,141 passed, zero failed/ignored across 45 summaries; clippy PASS with zero diagnostics.
- Final-round all operation dispatched native build/CTest through coordinator request `qbuild-20261024-145415-build-ctest`: build PASS 46/46 edges (2.2 s, sccache 30 hits/0 misses); CTest PASS 81/81, zero failures (98.1 s); overall qbuild PASS. Session 26610 was collected to terminal exit 0. No duplicate verify was launched.
- Earlier stable-source gate also passed: request `qbuild-20261024-131855-build-ctest`, build 262/262, CTest 81/81 (928.47 s).
- Evidence: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. Engine source is unchanged since `d230f041b`; later commits only preserve lane status.
- Initial Bash q.py transport failure remains in `build-runs/q/20261024-124722-31960.log`; final-round wrapper failure is preserved in `build-runs/q/20261024-145255-14932.log` (Bash CreateFileMapping access denied before the gate started). Direct qbuild all then completed successfully. Tested source is unchanged since d230f041b.

## Game result and external blocker

Run [498](../runs/modelinst-static-2032-20261024-a-653.md), `modelinst-static-2032-20261024-a`: terminal harness no-load, city never loaded within 1,200 s; wall 2,717 s including hung end screenshot. Coordinator confirmed harness failure, not code. Reference warnings, foreign exceptions and Rust panics were zero, but no modelinst measurement was possible.

Run [499](../runs/modelinst-static-2032-20261024-b-654.md), `modelinst-static-2032-20261024-b`: the one authorized retry uses the same train193 switches plus `modelinst_shadow=1`, turbo to 2032-01-01, seconds 600, timeout 1,200. Slot 5 running marker since 14:50:59, queue pid 27484. At 15:29:04 there was still no verdict or plugin log, beyond timeout plus 15 minutes. The request remains outstanding and a later result for this unique label remains valid.

Exact `<label> <clean>/<total>` line: unavailable, not 0/0 or 73/73. No reached date, shutdown, ref-warning count, resync or house-shadow measurement is available for run 499. No coverage credit or shared regress-profile edit is requested. MI14 remains open; no new owners or questions were started this round.

Wait sessions 27103, 30003, 54090 and 99630 each reached terminal nine-minute timeout. No wait or gate remains active. The game request was never canceled or duplicated. This is the brief's WIP exception: nothing at all could be measured in a loaded city. The passing native gate does not replace that requirement.

## Next steps

1. Coordinator: inspect the stalled slot 5/queue and collect the existing `modelinst-static-2032-20261024-b` verdict; do not discard a completed result or duplicate the pending request.
2. Run `python build-runs/summarize-modelinst.py modelinst-static-2032-20261024-b` after the log arrives, check shutdown/ref warnings and house lines, update run 499 with the exact `<label> <clean>/<total>` measurement, then queue ready for measured work if clean.
3. Resolve MI14 before implementing live or remaining-owner comparisons. Keep those claims open and the switch default-off.

Manifest for this round: `C:/Users/tlori/FreeCity4/build-runs/run4-glue-modelinst-shadow11-r3-manifest.json`.

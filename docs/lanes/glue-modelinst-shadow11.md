| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-modelinst-shadow11 (finite static fixture) | `Codex` / `glue-modelinst-shadow11` (engine lane) | in progress; previous gate cargo 7141/0, clippy PASS, CTest 81/81; run 498 harness no-load; retry run 499 pending; MI14 open; public ABI unchanged | (1) Collect modelinst-static-2032-20261024-b. (2) Final gate and measured report. (3) Queue measured work ready; retain MI14. |

## Handoff

- Goal: finish finite static shadow measurement; no new owners or questions.
- Done: source d230f041b; previous gate passed; prior run 498 ended no-load (harness failure confirmed by coordinator).
- New request: modelinst-static-2032-20261024-b, same train193 switches plus modelinst_shadow=1, turbo to 2032-01-01.
- Next: wait once via build-runs/wait-verdict.py, summarize, run final qbuild all, commit report and queue full HEAD ready for measured work.
- MI14 stays open; no live or whole-owner coverage claims; public ABI unchanged.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-modelinst-shadow11-r3-manifest.json.

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
- The same operation dispatched native build/CTest through coordinator request `qbuild-20261024-131855-build-ctest`: build 262/262 edges; CTest 81/81, zero failures (CTest log total 928.47 seconds). No second verify was launched.
- Evidence: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. Engine source is unchanged since `d230f041b`; later commits only preserve lane status.
- Initial Bash q.py transport failure remains in `build-runs/q/20261024-124722-31960.log`; direct cargo and subsequent full gate passed. Built DLL: 20,934,656 bytes, 2026-10-24 13:46:33 local.

## Game result and external blocker

Run record: [modelinst-static-2032-20261024-a-498](../runs/modelinst-static-2032-20261024-a-498.md). Request `modelinst-static-2032-20261024-a` uses the train193 house switches plus `modelinst_shadow=1`, turbo and `end_date=2032-01-01`, timeout 1,200 seconds. The only result is a running marker: slot 4 since 2026-10-24 14:03:53, queue pid 27484.

At 14:39 local there was still no verdict or plugin log, beyond the configured timeout plus 15 minutes. `docs/harness.md` lines 120-122 directs returning blocked in this situation. No exact/total line, reached date, clean shutdown, reference-warning count, resync measurement or house-shadow measurement is available. The 73/73 target is unverified; no coverage credit is requested.

Wait history is preserved: sessions 79080 and 41855 timed out; inherited session 22617 was unavailable on resume; replacement session 55162 reached terminal timeout; session 58313 was interrupted to a terminal exit after the documented stalled-queue threshold. No wait or gate remains active in this transport. The game request itself was never canceled, replaced or duplicated. A later completed verdict for this exact label remains valid.

## Next steps

1. Collect the existing verdict with `python build-runs/wait-verdict.py build-runs/game-results/modelinst-static-2032-20261024-a.verdict`; do not resend solely because these waits timed out. Coordinator should inspect the stalled queue if the result remains absent.
2. Run `python build-runs/summarize-modelinst.py modelinst-static-2032-20261024-a` after the log arrives, check shutdown/ref warnings and house lines, update run 498 with the exact `<label> <clean>/<total>` measurement, then change the merge entry to ready only for measured work.
3. Resolve MI14 before implementing live or remaining-owner comparisons. Keep those claims open and the switch default-off.

Manifest for this round: `C:/Users/tlori/FreeCity4/build-runs/run4-glue-modelinst-shadow11-r2-manifest.json`.

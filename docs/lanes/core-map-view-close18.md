| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-map-view-close18 (partial map-view owner closure) | `core-map-view-close18` / engine lane | Partial QV5/QV11/QV16 implementation at 49b99359d, integration-path fix e706923b1; focused 2/2, corrected native CTest 81/81; qbuild all pending. Owner closure incomplete. ABI unchanged; no new questions. | (1) Finish qbuild all; (2) implement QV1/QV2/QV3/QV14 and vectors 35–48; (3) close every owner row. |

## Report

Merged `re-map-view-qv18` (`2ef6fa4ba551c32bb1fdda490a580225a697e201`) as
`45e79361d`. The generated QUESTIONS total was the only merge conflict; summing
the merged rows gives 658 open and 1563 answered.

Implemented QV5's scale-presence rules: absent/invalid scales use factors 256,
explicit valid scales use the unsigned/signed quotients, invalid raw scale values
remain stored, negative products shift arithmetically, and each selection resets
the factors. Word grids now feed the byte drawing path. Added conformance vector
1 coverage for defaults, explicit 256/1024, negative values and selection reset.

Implemented QV11's empty default/unknown layers, shared layer-3/4 rendering and
layer-6 debug grid construction from ordered signed triples. Duplicate entries
overwrite, multiplication wraps to signed 16 bits, unavailable data draws nothing,
and debug drawing suppresses interpolation without changing saved configuration.
Vector 34 covers these branches, including the debug override and empty sequence.

Resolved QV16 using its authoritative truncated-step rule: vector 21's 2-to-3
centre is 0; the separate exact-half 2-to-1 reduction remains 2. Updated the
module README's QV5/QV11/QV16 closure entries and obsolete test expectations.

Validation: focused vectors PASS 2/2 (44 filtered; compile 25m56s on the contended
host). First native request `qbuild-20261024-214427-build-ctest` built 267/267 edges
but CTest failed 80/81 because the new test module needed an explicit path
attribute. Fixed in `e706923b1`; preserved failed logs under
`build-runs/failed-first-fc4verify-*.log`. Corrected request
`qbuild-20261024-215541-build-ctest` PASS: build 46/46 edges, CTest 81/81, 96.2s.
The required qbuild all operation is still running; workspace/clippy are pending.

The credit target is **not complete**. QV1 (§§2/3.1, vector 35), QV2 (§4.1 traffic
and marked cells, vectors 36–39), QV3 (§§1.2/3.2.1/3.3, vectors 40–44), and QV14
(§5.2 inherited PC/RC ring precision, vectors 45–48) remain implementation work.
All have published answers; no new question was filed or unresolved spec gap
claimed. The context-stop rule ended new feature work at roughly two thirds.
No plugin/C ABI wiring, ABI export changes, coverage-map edits or game runs.

## Handoff
- Goal: close every cSC4WinMapView owner row in engine/core/src/map_view, without plugin, capi, ABI or coverage-map edits.
- Worktree: C:/Users/tlori/FreeCity4/.claude/worktrees/core-map-view-close18; detached HEAD intentionally reserved by supervisor.
- Done: merged re-map-view-qv18 (2ef6fa4ba551c32bb1fdda490a580225a697e201) as 45e79361d; generated QUESTIONS conflict resolved from merged rows (658 open / 1563 answered).
- Done: 49b99359d implements QV5 scale-presence factors/conversion and retained invalid scale; QV11 empty/default/shared/debug layers; QV16 vector-21 correction and closure row. e706923b1 fixes the integration module path.
- Added close18_vectors.rs tests v01 and v34; adapted obsolete 16-bit expectations and throttle test. Formatting is applied through apply_patch, using rustfmt --check diffs only.
- Next: collect qbuild all operation session 27498 to terminal (build-runs/qbuild-all-final-1.log), then report/queue the measured partial work per the run-5 context-stop rule.
- Remaining owner work: QV1 display/none reset (vector 35); QV2 traffic reconstruction/markers (36–39); QV3 panels/memories/reset/commands (40–44); QV14 inherited PC/RC ring arithmetic (45–48). All answered, no new question needed.
- Gates: focused cargo PASS 2/2, 44 filtered (25m56s compilation on heavily contended host), session 33160 completed. Corrected native qbuild-20261024-215541-build-ctest PASS: build 46/46 edges, CTest 81/81, 96.2s total; session 6612 completed.
- Failed evidence: native qbuild-20261024-214427-build-ctest build PASS 267/267, CTest 80/81 (cargo_test E0583); preserved as build-runs/failed-first-fc4verify-{ctest,build,configure}.log. Source fixed before corrected native gate; workspace qbuild all was waiting on build-directory lock and is now progressing on corrected source.
- Earlier diagnostics: Bash wrapper Win32 error 5 (q/20261024-213040-2940.log); initial relative-path fmt failed, absolute-path fmt corrected and patched. All current owned files format clean.
- Game: no request needed for this core-only task and none outstanding. ABI unchanged; plugin/capi/coverage map untouched.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-map-view-close18-r1-manifest.json.

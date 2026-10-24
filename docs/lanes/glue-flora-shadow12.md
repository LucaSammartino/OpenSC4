| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-flora-shadow12 (flora owner shadows) | Codex / glue-flora-shadow12 | ready: source 69aeea81b; cargo 7212/0, clippy clean, CTest 81/81; flora12-20261024-r1 clean through 2032, ref_warnings=0; static 1/1, occupants 73/73, simulator months 73/73, registry 1/1 exact | Coordinator: credit all three flora owners using the recorded patterns; review existing flora_shadow regression-profile membership |

## Changes

Source commit `69aeea81b` extends the existing default-off `flora_shadow` comparison with distinct owner lines. `flora-static-shadow` compares the Rust-built static records against the original's blocks from the same exemplars; its mismatch count excludes simulator registry/order checks. `flora-occupant-shadow` compares native occupant observations against the Rust deterministic monthly rule; its count excludes simulator scalar checks and random appearances/disappearances. Both retain the original detailed registry/month diagnostics. No new ini key, C ABI export or ABI version change.

Missing captured static records now make a month INCOMPLETE instead of allowing an unchanged prediction. Tests exercise clean results, a changed static float, an altered occupant state and missing static data. Plugin startup describes both owner lines. The coverage report identified flora as three owners with verification partial; forbidden tools-side maps were not opened or edited.

## Validation

- Focused capi shadow test: 1 passed. Explicit owned-file fmt: passed.
- Stable final engine source: `qbuild.py all` PASS, workspace 7212 passed / 0 failed, clippy 0 diagnostics, native build 262/262 edges, CTest 81/81.
- Native work was queued by that same all operation as `qbuild-20261024-144618-build-ctest`; no duplicate verify was launched. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. The native tree-change note was docs-only commit `3d1728678`; engine source remained identical to `69aeea81b`.
- Run [511](../runs/flora12-20261024-r1-511.md): Konradshohe, original flora, `enable=simulator`, `flora_shadow=1`, turbo, requested end 2032-01-01; clean verdict at 2032-01-13, ref_warnings=0, exceptions=0, RTC=0, stalls=0, incomplete lines=0. DLL SHA256 prefix `0e9fc944`.

Exact clean/total lines (counts are log lines, not record counts):

```text
flora12-20261024-r1 1/1 flora-static-shadow; compared=23 mismatches=0
flora12-20261024-r1 73/73 flora-occupant-shadow; compared=190213 mismatches=0
flora12-20261024-r1 73/73 flora shadow month; compared=190213 mismatches=0
flora12-20261024-r1 1/1 flora shadow registry; compared=23 mismatches=0
```

Static line: `flora-static-shadow: compared=23 mismatches=0 owner=cSC4StaticFloraData`. Last occupant line: `flora-occupant-shadow: date=2032-02-01 compared=2587 mismatches=0 visited=203 owner=cSC4FloraOccupant`. Existing simulator month lines all have `mismatched 0`; registry has `differences 0`. Together the four patterns are `flora12-20261024-r1 148/148`. Full evidence: `build-runs/game-results/flora12-20261024-r1.log` and `.verdict`; bounded summary helper: `build-runs/summarize-flora.py`.

The file-event wait timed out despite the completed verdict; reading the same label then collected the valid terminal verdict. No duplicate game request was submitted. Turbo was exact, so no normal-speed diagnostic was required.

## Scope and follow-up

The existing spec exclusions remain explicit: random seeding/cluster outcomes, generator state, unused high flag bits and uninitialised key-0 flags/cost are not credited. The house run had propagation off; these lines establish exactness for the measured deterministic comparisons, not random coverage. Static record count includes 22 exemplars plus key 0.

Questions: none; no spec contract blocked the measured comparisons. Coordinator should credit `cSC4FloraSimulator` with the existing month/registry patterns, `cSC4StaticFloraData` with `flora-static-shadow`, and `cSC4FloraOccupant` with `flora-occupant-shadow`. Coverage scripts were not run because tools/ is forbidden to this lane. Keep shadows default-off; the existing `flora_shadow` key is the only required regression-profile key. Tools-side map/profile updates and any merge from main belong to the coordinator; this detached worktree did neither.

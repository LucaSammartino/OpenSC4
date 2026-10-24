| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-overlays-shadow5: terrain overlay full observation | Codex; glue-overlays-shadow5 (detached worktree) | in progress, round 2 | Implement OV12 observation/replay; final gate and game through 2032 |

## Handoff
- Goal: default-off record/cell/texture shadow to OV12; measure available contracts through 2032.
- Merged re-overlays-ov12 at e30d7b66a, preserving both sides of CMake/config/lib.rs additions, then merged main.
- Implemented native bounded copies of all three arenas/free queues, live IDs/groups/placement, scalar/placement getters, ordered static lists, texture binding metadata/sharing and source implementation tokens. Rust diagnostic parser checks graphs/membership/getters, with no native answers seeded into OverlayManager.
- Focused Rust OV12 A-L plus corruption test: 13/13 PASS. Initial J fixture index typo failed 12/13; corrected and rerun. q.py shell bootstrap failed before Cargo; direct PowerShell Cargo worked.
- Filed OV13 (capture boundary for future full independent history replay); current snapshots explicitly report unavailable replay and image bytes per OV12.
- Implementation commit: f2bdcff61. Required pre-gate main merge was already up to date.
- Full gate PASS: workspace 6818/0/0; clippy 0 diagnostics; native build 212/212 (145.2s), CTest 71/71 (181.1s), gate total 330.2s. Queue qbuild-20261024-012924-build-ctest collected to terminal PASS.
- Next: game overlays-shadow5-r2-2032a through 2032. Request will be submitted with this milestone; collect via event-based build-runs/wait-verdict.ps1 (no qwait).
- Merge main again immediately before final all gate.
- No game request outstanding; new unique label required. Scan docs/runs for free number beyond 348.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-overlays-shadow5-r2-manifest.json.
- Previous round evidence below is retained until round 2 report replaces it.

## Result and scope

Default-off `overlays_shadow=1` now compares production Rust overlay getters with all four original managers, without native writes, hooks, allocations, retained pointers or replacement. Implementation: `bd552f9e9`; load-readiness fix: `58f2d0907`. The private static-library transport adds no public C ABI export and does not change the ABI version.

Each sample compares 25 words per manager: shown, binary32 alpha, three colour words, draw-state byte, self-lighting, nine untouched placement words for handle -1, and nine for a direct ID. Colour slot 24 uses the spec's output-first thiscall rather than the incorrect SDK declaration; slot 26 uses the documented boolean return. Missing managers/core results count as failures. The load sample runs on the first framework tick exposing all four managers, outside native Init; monthly samples and shutdown totals follow.

**This is READY only for the measured getter subset.** It does not verify live overlay records, IDs/types/order, per-overlay parameters, cell indices, texture contents, draw matrices or render counters. OV12 asks for exact native arena metadata/enumeration and independent replay inputs; the public interface has no enumerator, and table offsets/record sizes alone do not establish a safe walk. No full-overlay coverage credit is claimed. Nondeterministic exclusions within the measured probes: **none**. Blocked state is explicitly printed as `records=blocked(OV12)` rather than excluded as nondeterministic.

## Verification

Final stable source: `qbuild.py --wt <WT> all` **PASS**, including the coordinator-queued native verification (`qbuild-20261024-004704-build-ctest`). Cargo: **6696 passed, 0 failed, 0 ignored**, summed over test-result lines; Clippy: **0 diagnostics** with warnings denied. Native build: **89/89 edges**, 4.3 s; full CTest: **69/69**, 83.5 s; native gate total 92.2 s. The focused C API transport test passed 1/1; explicit-file Rust formatting passed. Logs remain in `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, and `fc4verify-ctest.log`.

- [Diagnostic run 341](../runs/overlays-shadow5-r1-2032a-341.md), label `overlays-shadow5-r1-2032a`: clean through 2032-01-13, ref_warnings=0; `72/73 lines exact mismatches=100 fields=7300`. PostCityInit preceded manager availability; all 72 monthly probes were exact. Evidence retained, followed by the load-readiness fix and a complete repeated gate.
- [Final run 342](../runs/overlays-shadow5-r1-2032b-342.md), label `overlays-shadow5-r1-2032b`: Konradshohe, turbo, clean through **2032-01-09**, **ref_warnings=0**, all harness problem counts zero. DLL SHA256 prefix `b4df96ae`, 57.907 run seconds, 77 s harness wall, slot 2. Load and 72 months each report `100/100 mismatches=0 managers=4`.

Exact final summary:

```text
overlays contract shadow: 73/73 lines exact mismatches=0 fields=7300 scope=sentinels records=blocked(OV12)
```

Both unique game verdicts were collected; no request remains outstanding. The final absolute-path event wait completed normally. The initial relative-path waiter was stopped after its already-completed verdict was read; that run was not discarded or repeated because of the waiter. The source correction, not the wait, required the second run.

## Questions and next steps

**OV12**, appended to `docs/specs/questions/overlay-manager.md`: exact arena enumeration, complete record/cell/texture layout and independent reconstruction inputs. Coordinator should regenerate the questions index, resolve this contract, and extend the shadow to live records before crediting full terrain-overlays verification. `tools/` and coverage maps were not edited under the clean-room restriction; coordinator owns any reviewed regression-profile addition. The key remains default-off.

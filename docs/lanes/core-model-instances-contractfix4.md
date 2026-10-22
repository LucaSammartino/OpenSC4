| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-model-instances-contractfix4: MI3-MI10 corrections | Codex / `core-model-instances-contractfix4` (engine lane, detached supervisor checkout) | ready; cargo 4718 PASS; clippy 0 diagnostics; ctest 68/68 PASS; 42 model tests; MI11-MI13 filed; ABI unchanged; game n/a | 1. Supervisor publish/merge bounded corrections. 2. Resolve MI11-MI13 and retain native exclusions. |

## Report

Corrected the existing portable model-instance operations against revised model-instances MI3-MI10,
graphics-formats §§3.9/3.12 and view-utilities §7. Base was detached `1c4c54d2` in the reserved worktree.
This is the authorized native-answer correction phase, with no restart, additional lane round, or fourth phase.

Commits: `965e8949` initial contract corrections; `f778bcf4` independent vectors and ownership boundaries;
`96cd9dd4` Rust 1.75-compatible atomic updates. The final full gate ran on `96cd9dd4` with no edits during it.
This final report commit changes documentation only.

Changes in the seven `engine/core/src/model_instances/*.rs` files:

- Basic/static defaults use properties 2. Draw records preserve all 20 bytes; only byte 0 receives the special
  blend flags. Four default float words are 1; unwritten constructor bytes 2..3 explicitly choose zero.
- Transform counters are wrapping u8 values. Get increments the destination; Set increments the instance,
  ignoring the source counter. MakeIdentity tests/clears the actual kind byte and does nothing for kind 0.
- Bounds use scaled endpoint contributions through the full row matrix, explicit binary32 stores and strict
  product selection. Kind 0 does not bypass the matrix; inverted sentinels reorder into enclosing boxes.
- Picking compares positive Euclidean distance to the rounded hit point. Equal/NaN distances do not improve
  nearest; first improvement ends a mesh, while later meshes can still improve it. Behind-origin hits remain valid.
- Processor binding refuses absent, empty, unsupported-format and multi-frame containers without retaining either
  reference. Rebinding restores originals first. Destruction releases the instance, restores exact original block
  identities, releases the retained container if needed, then discards arrays. ReleaseModel preserves arrays and
  a weak borrowed container for later restoration; it requires an external owner. Repeated/no-container releases
  return a portable error, without claiming native idempotence or safe dangling-pointer access.
- Sort restarts from container order and scans from the front. Equal, signed-zero and unordered scores insert
  before existing entries. Native-reference-only SetModel binding is separated from the portable frame-provider
  convenience; material fields remain instance-local and same-pointer binding still inspects them.
- Playback uses wider transients with published binary32 accumulator stores. NaN returns 0 without stepping;
  infinity takes the single-step catch-up branch. Primitive helpers subtract without saturation and iteration
  stops for nonpositive signed counts. Checked table sizes and malformed-mesh skips remain portable choices.
- Counts distinguish basic u16, static/animated u32, atomic resource u32 and locked factory u32 policies.
  Construction starts at 0; counted handles start at 1. Instance/factory zero-or-one release destroys, while
  resource zero release wraps to 0xFFFFFFFF without native destruction. Basic overflow destruction invalidates
  surviving safety handles. Arc lifetime protection and eventual Rust reclamation are explicitly portable,
  rather than claims about native overflow survival or resource-underflow leaks.
- MI7 upward-fill behavior remains intact, with unsigned normalization, nonzero wrap and I=0 failure tests.
  Wrapped-zero sheet keys conservatively fail pending MI13; that boundary is not claimed native-exact.

## Tests and measured results

The module now has **42 passing tests**, adding 11 tests and correcting the old assumptions rather than
preserving the previous 31-test expectations. The 20 independent vectors are represented as follows:

- 1-6: base delegation, static texture sort value, animated processor refusal, area-weighted normals,
  unequal mesh ordering and shared-sheet tests.
- 7/8/15: `published_defaults_draw_record_and_material_isolation`; vector 15's second copy is tested at
  frame-builder stage, qualified by MI12. Both independently bound instances also apply their own changes.
- 9: destination-local Get/Set counters and MakeIdentity wrap/no-op tests.
- 10: native count widths, zero releases, 32-bit wrap, and real basic-overflow destruction exactly once.
- 11: owner callback observations distinguish basic release-before-retain from static retain-before-release;
  same-owner and clear/destruction counts are checked.
- 12: all-own/mixed downward/upward sheet scales, unsigned wrapping and I=0 failure.
- 13: restart, reversed ties, signed zeros and the distinguishing (2,NaN,1) order.
- 14: eligibility refusals, same-instance reconversion, ReleaseModel transfer, restoration on drop/rebind,
  retained-versus-borrowed counts and no second container release.
- 16/17: exact asymmetric negative-scale box and Euclidean distance/sign/ties/nonfinite/mesh-order vectors.
- 18: nonfinite playback plus a transient below 1 that rounds to 1 in accumulator storage.
- 19/20: negative primitive helper counts/no visits, table sizes 36/44, checked wrapped-count rejection,
  resource construction/query counts, failed-query output preservation and factory-failure null output.

Final `qbuild all`: **4718 passed, 0 failed, 0 ignored; clippy exit 0, 0 diagnostics; native build 4/4 edges
PASS (78.5 s); full ctest 68/68 PASS (150.5 s)**. Logs: `build-runs/qbuild-test.log`,
`qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. All seven touched Rust files passed
qbuild rustfmt checks; `git diff --check` passed. Quick checks passed 2721 then 2732 core tests plus dependent
compilation and native builds.

Initial baseline Rust 4707/clippy/build passed, but baseline ctest overlapped edits and was invalid (67/68,
cargo_test compile failure); baseline-prefixed logs preserve that fact. A formatting helper's Windows filename
heading mistake was removed before the expanded quick PASS. The first stable full run passed Rust/native/ctest,
but clippy rejected try_update's Rust 1.95 requirement. The committed compare-exchange loop fixed the Rust 1.75
MSRV issue, and the final repeated full gate above passed. No redundant clean baseline was rerun.

## Questions, scope and next steps

Appended **MI11-MI13** only to `docs/specs/questions/model-instances.md`:
MI11 asks for required x87 precision control and remaining point/pick stores (f64 is an explicit numeric limit,
not bit-exact extended precision); MI12 resolves vector 15's second-instance stage; MI13 asks for exact interior
and initial wrapped-zero sprite-key behavior. The generated question index still advertised MI3, but the revised
spec already uses MI3-MI10, so those IDs were not reused. Coordinator should regenerate the index.

No whole-native-owner/family, runtime, shadow, replacement or default promotion is claimed. Animated convenience
slots 0-8, processor assignment, generic persistence and sprite-instance binding/rendering remain excluded.
No plugin, CAPI, CMake, map, default or ABI edits; no C exports changed. No game requests or game claims are
appropriate for this pure-core task. No native/analysis sources, external communication, agents or network used.
The unavailable quiet Bash launcher required bounded PowerShell reads; all build/test checks used qbuild.

Supervisor may publish this detached HEAD and enqueue the bounded corrections; preserve the exclusions and
resolve MI11-MI13 before claiming exactness beyond the measured vectors. No new implementation round requested.
Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-model-instances-contractfix4-r1-manifest.json`.

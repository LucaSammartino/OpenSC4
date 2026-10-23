| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view-utilities5 (SC4ViewUtilities owner closure) | Codex / reserved `core-view-utilities5`, detached HEAD | ready for review: settled portable contracts mapped; cargo 5875/0, clippy clean, CTest 68/68; native numeric closure open VU12/VU13/VU15 | Coordinator: regenerate question index, review/land; resolve numeric questions before claiming native whole-owner completion |

## Report

Reused `engine/core/src/view_utilities` and audited it against all sections of
`docs/specs/view-utilities.md`. The module README now has an **Owner closure**
table with a row for every section/subsection, code items, covering tests and
explicit open questions. It retains the existing 50-entry family inventory;
mapped portable bodies are not claimed as native-exact byte coverage.

Added explicit §12 cached getters: `get_frustum_aabb`,
`get_draw_ground_corners`, and `get_camera_view_angles`. They work without a
validity gate, retain stale geometry after camera failure, and do not query
camera rays. The angle getter evaluates cached zoom/rotation even before the
first camera update, unlike the constructor-zero stored angle snapshot.

Added getter regression tests and the literal §11.5/16 city and §11.20 mesh/model
order vectors. Added §4.4 conversion ties and isolated the block helper integer
stage to cover §11.18 with n=INT_MAX without silently rounding its input to 2^31.
The README maps all 21 conformance vectors; vector 11 uses the spec-permitted
checked failure adapter, and vector 18 entry-point coverage is explicitly open.

All settled portable algorithms are mapped. Full native numeric owner closure
is **not claimed**. The remaining open portions are:

- **VU12 (existing): §2.2 and arithmetic in §§2–7**, including remaining corner,
  projection, predicate and transform evaluation/store details and inherited
  numeric mode. The documented portable evaluation policy remains in effect.
- **VU13 (existing): §§7.1/12**, generic index formats 6/7 outside the explicit
  finite bias range. Those checked decoder inputs return None, preserving output.
- **VU15 (new): §§4.2/4.4/11.18/12**, block helper scalar input width. INT_MAX in
  vector 18 is not representable by the existing f32 entry adapter. The specified
  wrapping integer stage is covered; no wider native signature is guessed.

VU15 is appended to `docs/specs/questions/view-utilities.md`. Only that spec
question file was edited; the coordinator must regenerate the question index.

## Verification

- Owned-file formatting: PASS for `mod.rs` and `tests.rs` using absolute paths.
  An initial relative-path invocation failed path resolution, then was corrected;
  it did not find a Rust formatting or compilation failure.
- Focused view utilities tests: **46 passed, 0 failed**;
  `build-runs/q/20261023-132012-9632.log`.
- Required `qbuild.py --wt <WT> all`: **PASS**, collected session 37922 to its
  actual terminal exit=0; one full gate, no duplicate/rerun.
- Workspace cargo: **5875 passed, 0 failed, 0 ignored**, summed across 27
  `test result:` lines; `build-runs/qbuild-test.log`.
- Clippy with `-D warnings`: **PASS, 0 diagnostics**;
  `build-runs/qbuild-clippy.log`.
- CMake: **PASS, 217/217 edges**, reported 91.5 s;
  `build-runs/fc4verify-configure.log`, `build-runs/fc4verify-build.log`.
- Full CTest: **68/68 passed**, reported 165.8 s;
  `build-runs/fc4verify-ctest.log`. Overall verifier reported PASS in 1161.4 s.
- The verifier annotated “tree changed during the run”: commit `53d07251`
  changed only this lane report and the module README while the gate ran.
  The tested Rust source remained exactly that of `a18c9282` throughout.
- Game runs / shadow: **n/a**; no plugin/capi wiring, ABI exports, game requests,
  default/replacement registration or coverage-map edits.

Commits: `27dd3666` initial audit checkpoint; `a18c9282` implementation/tests and
closure inventory; `53d07251` documentation checkpoint of the active full gate.
No asynchronous operation remains outstanding.

Next: coordinator review/land and question-index regeneration; obtain VU12,
VU13 and VU15 answers before claiming native whole-owner numeric completion.

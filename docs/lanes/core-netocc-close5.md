| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-netocc-close5: core network occupant owner closure | Codex; core-netocc-close5 (detached HEAD; reserved branch core-netocc-close5) | ready: model instance subset, gate PASS | Review one owner closure; continue seven remaining owners |

## Report

Delivered subset: **1/8 owners**, `cSC4NetworkOccupantModelInstance`, for the
published network-occupants §§10–10.3 contract and §13 vector 24. The module
README has the Owner closure table mapping operations to code and tests, plus
an explicit inventory of the seven unclosed owners. No capi/plugin/ABI changes.

Implementation commit: d1ca36d4. `FlipAndLight` now requires and dispatches the
foreign `cS3DModelProcessor::LightModel` primitive, after winding/range changes
and flip-adjusted lighting transform construction, before ReleaseModel drops
the retained source arrays. Previously it merely recorded a pending request.
There is no default lighting service. Resource providers must supply it; the
network owner does not accept a foreign network-owner result. The generic
trait bound avoids trait-object upcasting, preserving the Rust 1.75 contract.

Added four discriminating tests: processor dispatch/mutation and source
lifetime ordering, main/second/option dispatch, mode-0 unsupported-cut boundary,
and conformance vector 24 (returned heap instance and four corner heights for
both diagonal choices). All **21 model tests** passed in the final gate.

### Validation

- Explicit-file qbuild fmt: PASS for model.rs, model/prebuilt.rs, tests_model.rs.
- Focused implementation test: 19/19 passed before the final two vectors were
  added; log `build-runs/q/20261023-115152-41712.log`.
- Final qbuild all: **PASS**, operation 66254 collected to terminal exit 0;
  Rust **5856 passed, 0 failed, 0 ignored** across 27 result lines; clippy
  **0 diagnostics** with warnings denied; CMake **217/217 build edges**;
  CTest **68/68**. Verification wrapper duration 1138.5 seconds.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-build.log`, `fc4verify-ctest.log`.
- Wrapper reported "the tree changed during the run": only the lane handoff
  commit 5d60ca91 was made after gate start. `git diff --name-only d1ca36d4
  5d60ca91` contains only this lane file. Engine source and build inputs stayed
  stable; no duplicate full check was launched.
- Game: n/a, no requests, no shadow or real-game claims; no ABI/plugin wiring.

### Questions and limits

Filed **NO32** in `docs/specs/questions/network-occupants.md`: missing base-owner
UV values, packed render states, zoom/predicate data and normal colour encoding.
The generated question index is left to the coordinator (engine lanes do not
run tools/ scripts or edit QUESTIONS.md).

Model-instance remainder: six-plane mode-0 cutting stays explicitly Unsupported
because §10.3 excludes that path (**NO29**); null/contained early-outs work.
**NO27** supplies no numeric processor constants: callers must provide them.
**NO28** concerns the foreign processor's arithmetic, reached through the new
required primitive. **NO30** remains a literal reading of the options/Clip
exception. **NO31** asks about unpublished native ctor/dtor effects; only the
specified resource/mesh/processor lifetime behavior is claimed here.

The other seven owners are deliberately **not closed**: base §§2.4–2.7 geometry,
static initialization, path-cache and drawing; prebuilt §2.8 model/path/record
and native allocation-failure branch of §10.1; underground §§3.1–3.2 geometry,
wall helpers, lifecycle and drawing; connection §3.6 state/lifecycle/record;
line §5 live/record reconciliation (NO13) and base shutdown forwarding; pole
§§4.1–4.4 object/connection/lifecycle/record; pylon §§6.1–6.3 state, lifecycle,
drawing and record. Existing helper fragments do not earn closure claims.

Next: coordinator review/claim the model-instance subset, regenerate questions
index, and schedule remaining owner implementations/spec answers. No operation
or game request remains outstanding.

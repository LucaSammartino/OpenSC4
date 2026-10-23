| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-netocc-close5: core network occupant owner closure round 2 | Codex; core-netocc-close5 (detached HEAD) | wip: 0 new owner closures; model remains 1/8; Rust 5867 passed, clippy clean, CTest 68/68; NO33-NO37 and UG5 filed; ABI unchanged | Answer spec gaps for pole 4.1-4.4, pylon 6.1-6.3 and connection 3.6, then complete owners |

## Report

Round 2 delivers **zero additional whole-owner closures**. The Round 1 model
closure remains **1/8 overall**, with all its commits retained. Pole, pylon and
connection contain useful published contracts, but their required missing or
ambiguous behavior prevents closure. The module README now gives one row per
requested spec section, its code/tests, and every open remainder. No new
Unsupported implementation is used to conceal a missing algorithm.

### Changes

- `0b6285c2`: pole semantic state and connection views, two connection-point
  table selections, shared control initialization once, signed int64 static-word
  demolition quote, and the published Init state/resource prefix. No fabricated
  wire geometry, native pool, cost formula or conflicting record mapping.
- `4feba600`: pylon v5 write/v3-v4 read with both transforms and references,
  call/drag state, published message/tick subscriptions and shutdown order.
  Legacy decoding and source-offset choices are explicitly literal, pending NO37.
- `4feba600`: underground connection seven-wall record using the documented
  24-byte vertex helper, raw scalar/reference tail, hidden triangles,
  subscriptions and shutdown. Runtime reference fields remain separate from
  the unresolved serialized view mapping.
- No capi/plugin wiring, C exports, ABI version, game files or shared coverage
  map changes. Detached HEAD commits only; supervisor publishes the reserved branch.

### Validation

- Focused tests: pole **3/3**, pylon **6/6**, connection **2/2** (11 new tests).
  Logs: `build-runs/q/20261023-122856-29052.log`,
  `20261023-123534-20004.log`, `20261023-123947-13068.log`.
- Corrected failures remain separately preserved: private codec import in
  `q/20261023-122714-37508.log`, and missing SaveRegistry in the pylon test fixture
  in `q/20261023-123414-30300.log`. Both were fixed before the full gate.
- Explicit-file formatting: **PASS**, seven Rust files. `git diff --check` clean.
- Required `qbuild all`: **PASS**, single operation **91903**, collected to
  terminal exit **0**. Stable engine source was `4feba600`; no source or tracked
  documentation edits during the gate. Rust **5867 passed, 0 failed, 0 ignored**
  across **27** result lines; clippy **0 diagnostics**, warnings denied;
  CMake **4/4** incremental edges (66.1 s); CTest **68/68** (157.9 s).
  Verification wrapper: **1128.1 s**. No duplicate full check was launched.
- Full logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-build.log`, `fc4verify-ctest.log`.
- Game: **n/a**, no requests or shadows; these core contracts are not wired
  into the game. No asynchronous operation remains outstanding.

### Open sections and next steps

1. **PowerPole network §§4.1-4.4 remain open.** NO33 requests constructor,
   connection, geometry/rendering, resource and native-pool contracts; NO34 the
   exemplar-to-int64-cost formula; NO35 the record nesting/live offsets and
   post-read resource reconciliation. Conformance 2 has coverage, 4/16 remain
   open, and 17 covers static words but not the missing formula.
2. **Pylon network §§6.1-6.3 remain open.** NO36 requests state, geometry,
   texture/rendering and setter contracts; NO37 resolves live aliases, reference
   ownership/IIDs, legacy interpretation and placement effects. Conformance
   2/19/20 have executable published-contract coverage, not whole-owner closure.
3. **Connection network §3.6 remains open**, together with underground §8 UG1
   and new **UG5**: missing creation/callback/render-state/model contracts and
   raw/live reference reconciliation. Underground §§1,4-9 are audited in the
   README; subclass-specific conformance is not counted as base-owner closure.
4. Spec writers answer **NO33-NO37** in `docs/specs/questions/network-occupants.md`
   and **UG5** in `docs/specs/questions/underground-occupants.md`; coordinator
   regenerates the question index. Complete the open algorithms before claiming
   any new owner in coverage. Re-enqueue this round as **wip**.

## Round 1 record (retained)

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

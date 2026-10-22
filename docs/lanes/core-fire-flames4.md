| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-fire-flames4: portable fire flames | Codex / `core-fire-flames4` (engine lane, reserved detached HEAD) | Ready portable visual bodies; cargo 4316/0, clippy 0 diagnostics, ctest 67/67; FF2-FF4; no complete-owner credit, ABI or game promotion | 1. Publish missing inventory/lifecycle and resolve FF2-FF4. 2. Complete native owners before coverage credit or binding. |

## Report

Implemented `engine/core/src/disasters/flames`, exported as `disasters::flames`.
Substantive commits: `eb429f15` (published bodies and audit/questions),
`ae7adea9` (discriminating boundaries, formatting, disasters namespace).

### Covered and missing inventory

The full checklist is `engine/core/src/disasters/flames/README.md`.

- Covered published visual/helper bodies: Flame Read, Update, UpdateTransform;
  triangle construction/classification; GetPointsOnModel; fire CreateFlames,
  SetStage, Init and ViewUpdate. These run owned core algorithms, not callbacks
  to native owner methods. Published-domain coverage is subject to FF2-FF4.
- Owned behavior: old-effect stop/release before creation, transform/start order,
  first-primitive sequential triangle walk, cumulative local twice-area selection
  with >= boundary, biased barycentrics, normal offset before instance transform,
  independent clock-seeded model placement, inclusive T+S+1 box flames,
  flora/top/side branches, radian yaw, changed-stage ordering and model-only view
  repositioning. Getters use immutable borrows, without RefCell.
- Reused primitives: existing RzRandom, modern/legacy Xform readers, stream types
  and vector math. Camera changes only widen existing math visibility to the
  crate. No new dependency, format codec, ABI, plugin or coverage-map edit.
- Missing: the brief counts 2 owners/16 functions/3513 bytes, but this spec
  identifies only nine visual/helper bodies. Native construction, destruction,
  made/visibility transitions and other counted bodies are not fully published.
  Constructor flags are explicit inputs; unsupported stages and malformed input
  cases have portable diagnostics rather than invented native contracts.
- **Neither native owner receives complete-owner credit.** This visual component
  also does not implement the entire fire occupant from disasters §5.11. The SDK
  labels for hosing/family differ from the behaviour spec; no ABI is inferred.

### Measured checks

- Coordinator baseline reused as instructed; no redundant pre-edit full gate.
- Focused `qbuild quick`: PASS, 2443 core tests / 0 failed, dependent crates
  compile-only PASS, CMake 206/206 edges. That run observed the documentation
  milestone commit; the final gate below used a stable committed tree.
- Final `qbuild all` on `ae7adea9`: **PASS**. Workspace 4316 passed / 0 failed /
  0 ignored; core 2449 passed. Clippy `-D warnings`: exit 0, zero diagnostics.
  Native build 4/4 edges; full ctest **67/67**, 0 failed.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-build.log`, `fc4verify-ctest.log`.
- `qbuild fmt`: PASS on all eight touched Rust files (absolute paths required).
  `git diff --check`: PASS. No final `lib.rs` diff or unrelated reformat.
- 27 new Rust test cases use independent published vectors plus boundary/failure
  checks: effects/ref ordering, both transform record versions and every truncated
  prefix, yaw/side tables, exact cumulative boundary, area across scaled instances,
  biased transformed point, reseeding, partial triples, malformed/degenerate data,
  box extra count, top clamping/Gaussian ordering, flora RNG isolation, stage and
  view lifecycle. No conformance-runner extension was needed for this core API.
- Game runs: **n/a**, core only. No live binding, shadow/replacement claim or
  request outstanding; no real-game reference-warning verdict claimed.

### Questions and next steps

Appended to `docs/specs/questions/fire-flames.md`:

- **FF2:** reconcile full owner/body inventory, construction/destruction flags,
  Init discrepancy, initial/out-of-table stages, return/failure contracts.
- **FF3:** partial triples, primitive offsets, invalid mesh/zero-area behavior,
  generator class and exact numeric store boundaries.
- **FF4:** rotation signs/trig/mask/counter precision, box draw order and hiding.

FF1 already existed; its published z-normal wall predicate is retained literally.
The coordinator must regenerate the question index: this engine lane cannot run
`tools/` or edit the generated index. Resolve the missing native contracts before
complete-owner credit or a live adapter; the passing portable patch can land
independently. No further round, merge, push or branch/ref update was performed.

## Coordinator source-phase review

Bounded portable code/findings publication only; FF2-FF4 remain. No complete-owner, shadow or replacement credit is granted for this partial phase. Preserve the parked original worktree and existing round budget. Any engine change requires the joint full native gate and current default-union regression.

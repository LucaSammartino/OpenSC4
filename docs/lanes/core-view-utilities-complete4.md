| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view-utilities-complete4: portable view family reconciliation | Codex / `core-view-utilities-complete4` (engine lane, detached HEAD) | WIP: 50 distinct catalogue mappings/19748B, not implemented-byte credit; 44 utility tests; stable gate on 1d5387d6 cargo 4718/0, clippy 0 diagnostics, ctest 68/68; VU12–VU13 open; ABI unchanged; game/shadow n/a | 1. Resolve VU12–VU13 before native numeric/whole-family credit. 2. Preserve candidate; no fourth round authorized. |

## Result and completion boundary

Coordinator source-publication review (2026-10-22): accept these committed portable corrections for merge-train integration after reviewing the changed owner inventory, explicit I/P/C/R boundaries, stable gate evidence and clean tree. This acceptance does not close VU12/VU13, credit 19748 implemented bytes, or authorize another logical-family round. Preserve the parked worktree and all numerical/decoder exclusions; the train must gate the actual merged source.

Reconciled the portable view family with the landed VU2–VU11 contracts. The
README now maps all **50 distinct native entries / 19748 catalogue bytes**:
48 attributed entries / 19574 bytes, including the **free 78-byte rotation
helper**, plus the **two separately owned min/max helpers / 174 bytes**.
An independent local checker verifies the address sets, uniqueness and totals
against spec §12; it does not infer implemented bytes from rows.

**This remains partial. Do not credit a complete native owner, 50 exact bodies,
19748 implemented bytes, runtime replacement, or a shadow comparison.** The
README marks every row as integer/copy, portable arithmetic, checked boundary,
or Rust lifetime adaptation. Native numeric equivalence remains unproved and
the generic numeric index decoder has an explicit supported range (VU12/VU13).
The native iterator deleting-destructor heap/vtable ABI is represented by Rust
scope/drop with borrowed buffers, not claimed as an emulated native body.

This is the distinct completion phase after core-view-utilities4 and the native
answers, phase 2/logical 3. No reset, restart, fourth round, merge or push is
authorized by this report.

## Changes

- Signed remainder rotation and edge helpers, negative partial writes, wrapping
  visible-edge additions, untouched y word, and the free-helper attribution.
- Exact shipped angle words; exact integer accumulation for the extended-precision
  yaw expression before its float store; known scale reciprocal sources and
  first-vertex transform store asymmetry. Up uses the pre-normalized right vector.
  Portable f64/libm is explicitly not asserted equal to x87. Remaining arithmetic,
  including the full corner length-to-extent/store graph, remains numeric partial.
- Ground wrappers expose checked failure preserving outputs instead of invented
  successful NaNs for indeterminate native stack values. Direct failed eye rays
  preserve both vectors. Numerical invalid camera data still propagates; checked
  out-of-table zoom access is separately identified.
- Visible rectangles cast fresh TL/TR/BL/BR full-viewport rays independently of
  draw ground corners. Full and narrowed draw then visible then visible has ray
  counts **4/8/8**. Narrowed draw no longer fills the full-view ground cache.
  Missing narrowed camera retains old draw geometry/flag after entry stores.
- Previous far corners compound in perspective, and off-centre volumes retain
  the documented mismatch between corner extents and coordinate bounds.
- Bias pixel conversion is retained explicitly. Block conversion consumes the
  signed64 low word, accepts negative blocks, wraps arithmetic, and reports IDIV
  faults as None. Rectangle centres and their coordinate differences wrap in i32.
- Plane and two-sided triangle predicates use **0.01f** tolerance, inclusive
  barycentrics, native unordered comparison branches and negative t. Plane misses
  write zero; triangle rejection preserves output. Picking compares strict ordered
  **Euclidean distance to the rounded hit point**, including behind-origin hits.
- Ordered clockwise clipping preserves subject order, edge order, boundary
  duplicates, empty output and capacity truncation. Near-parallel indeterminate
  intersections return an explicit error; clipped bounds expose None.
- Strict first-point AABB scans retain first NaNs; cross helper exposes sequential
  alias writes through indexed storage. Separate min/max scans ignore NaNs and
  start at the specified finite bounds.
- Raw signed primitive counts/indices, eight-format checked index decoding,
  overlapping sufficient vertex storage, selected-group iteration and preserved
  triangle outputs on stop/error. Malformed references abort rather than skipping
  to later hits. Valid meshes stop at first improvement; valid models visit all
  meshes with a fresh transform each time. Checked model errors abort traversal.

The model-instances §§2.1/9.1 crossreferences describe this same picking operation.
No unrelated model-instance code or ownership changed. No CAPI, plugin, CMake,
tools, maps, ABI, runtime, default or shared registration files changed.

## Verification

| Check | Measured result |
|---|---|
| Initial `qbuild all` baseline attempt | Pre-change Rust/clippy passed; late CTest cargo check raced independent-test additions and failed on six then-unimplemented symbols; not a stable full baseline PASS |
| First focused quick | 2722/3; three stale old assumptions (parameter distance, forward-only hit, minimum stride); corrected against native prose |
| Focused stable `qbuild quick` | PASS: core 2730/0, dependents compile, native build 4/4 |
| Full run before MSRV/Clippy cleanup | Rust 4718/0, native CTest 68/68; clippy failed on argument count and const-from_bits MSRV; not final evidence |
| Final stable `qbuild all` on `1d5387d6` | **PASS: Rust 4718/0 (core 2732/0), clippy exit0/0 diagnostics, native build 4/4, CTest 68/68 (112.6s); fc4verify185.5s; no tree-changed marker** |
| Final `qbuild fmt` on five touched Rust files | PASS, exit0; only existing C:/Users/tlori canonicalization warning |
| Utility conformance tests | **44 total**, 11 added; independent expectations first committed in `da4fbb16` before implementation |
| Catalogue checker | PASS: exactly50 mapped addresses, 19748B, rotation78B, separate min/max174B |
| Whitespace/scope check | `git diff --check` PASS; only permitted source, README, question and lane-report files |
| Real game/shadow | n/a: pure portable core; no ABI/plugin registration or game requests; no ref-warning or exact-shadow claim |

Stable full logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
`fc4verify-build.log`, `fc4verify-ctest.log`. Initial evidence is preserved in
`build-runs/baseline/`; the pre-cleanup full run in `build-runs/pre-msrv-fix/`.
Independent address checker: ignored `build-runs/check-view-catalogue.py`.

Commits: `da4fbb16` independent expectations; `7dfbdeab` implementation and
questions; `f4967251` full mapping/focused edges; `1d5387d6` MSRV/Clippy cleanup.
The final report commit changes documentation only after the stable code gate.

## Questions and next steps

Appended **VU12–VU13** to `docs/specs/questions/view-utilities.md`, using the
index's next free VU12. VU12 requests supported inherited x87 mode and remaining
operation/store boundaries; VU13 requests numeric decoder bias/correction rules
outside the proven finite unit-spacing range. The decoder returns None without
writing outputs outside [-2^22,2^22); integer/unknown formats are implemented.
Only the permitted question file was edited; no question-index tooling was run.
Resolve these before native-bit or whole-family credit; retain this WIP candidate
and its evidence without automatically starting another round.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-view-utilities-contracts4: native view utility contracts | Codex / `re-view-utilities-contracts4` (RE spec lane; detached HEAD) | ready: VU2–VU11 answered; 50 entries/19748 bytes catalogued; 21 prose vectors; documentation checks PASS; cargo/clippy/ctest and game n/a; ABI unchanged | 1. Coordinator integrate specs. 2. Core owner reconcile native differences before claiming completion. |

## Report

Resolved VU2–VU11 in `docs/specs/view-utilities.md`, moved all ten original entries with answers to
`docs/specs/questions/answered/view-utilities.md`, and regenerated `docs/specs/QUESTIONS.md`.
No engine files were read or edited. No tools, ownership maps, ABI tables, HANDOFF or README were edited.

Commits: `f16fdbb5` proves rotation, failed-ray writes and independent cache sources; `ade418c6`
publishes numeric/geometry/resource contracts and the complete measured family checklist. This final
report commit includes final source review, coordinate-heading ordering and documentation validation.
Final follow-up records the narrowed-draw null-renderer/null-camera failure and retained stale state.

### Measured ownership

The native attribution audit, using main's export and metadata without invoking engine coverage,
finds 48 SC4ViewUtilities-attributed bodies / 19574 bytes. The separately mapped min/max helpers
add 2 / 174, so the complete requested family is **50 / 19748**. The previous 49 / 19670 count
omitted the free 78-byte rotation helper. No maps or denominators were edited. The checklist identifies
every address, export size, logical inputs/results, native rule and helper ownership. Three additional
nearby native getters are documented explicitly outside that attributed total; renderer/view-window
families are excluded. No whole-owner implementation credit is asserted.

### Material native contracts

- Discrete rotation uses signed remainder, leaves negative nonmultiples unchanged and preserves y.
- Direct failed eye rays preserve outputs; the ground wrapper ignores failure and consumes uninitialized
  stack values. Its coordinate is indeterminate, not guaranteed NaN or zero.
- Visible rectangles cast fresh full-viewport rays and have a separate cache from draw ground corners.
- Angle bits, native x87 trig/store behavior, reciprocal sources and unguarded invalid camera inputs
  replace assumptions about all-float32 formulas or invalid-zoom rejection. Reserved fields retain only
  their proved defaults; no unused-field semantics are invented. Off-centre volumes are not repaired.
- Pixel bounds use float-bit bias conversion. Block rounding and rectangle centres wrap in 32 bits;
  zero blocks can fault. No saturation or checked-result contract is supplied by the native helpers.
- Triangles are two-sided with 0.01f determinant tolerance and inclusive barycentrics, accept negative t,
  and can pass NaN comparisons; picking compares strictly smaller Euclidean distance. Plane tests use
  the same tolerance, accept coplanarity/negative t, and zero outputs on a parallel separated miss.
- City clipping fixes polygon winding, subject/edge order, boundary duplicates, empty output and the
  unchecked near-parallel intersection failure. Unsafe mesh references are not silently skipped;
  signed undersized counts stop iteration. Matrix rows, default-kind behavior and a native store
  asymmetry are documented, along with first-improving-triangle and all-mesh visitation order.

### Validation and limits

`questions.py index` and `questions.py check`: PASS. `git diff --check`: PASS.
Ignored `build-runs/check-view-spec.py`: PASS against the independently extracted native attribution:
exactly 50 unique rows, 19748 total bytes, all VU2–VU11 archived once, ordered coordinate headings,
and 21 prose conformance examples. These include independent non-unit/behind-ray geometry, determinant
tolerance, plane writes, rectangular city corners, off-centre volumes, ties/overflow, matrix orientation,
mesh ordering and cache-provider observations. They were not fitted to core vectors; no engine vectors
were read. Cargo/clippy/ctest and game/shadow runs: n/a for this documentation-only lane.

Evidence came from main's read-only `re_query.py` export, bounded decomp reads and PE constant words;
numeric claims were checked against instruction bytes. REA was unnecessary and was not called.
No game runs outstanding, no questions remain open for this spec, no ABI exports changed, and no ABI
row proposals were needed. Git emitted a packed-refs lock permission diagnostic while committing;
the detached commits were nevertheless created successfully, and no shared refs/config were changed.

Next: coordinator integrates the spec commits. The implementation owner should replace its documented
adapters/assumptions where these rules differ, then run implementation conformance and the required
joint gates. This lane supplies no runtime/default/shadow/replacement promotion.

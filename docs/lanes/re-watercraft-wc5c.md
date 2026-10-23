| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-watercraft-wc5c (remaining watercraft contracts) | Codex / `re-watercraft-wc5c` (RE, detached HEAD) | ready: incremental specs; WC3–WC5 remain open | Land verified contracts and 14 vectors; continue inherited/AI/grounding audit before owner closure. |

## Report

This lane adds behavior contracts to watercraft.md and narrows WC3–WC5.
It does not certify the whole owner complete. WC1/WC2 and WC6–WC8 were
already answered and were not re-investigated.

- `db7e11cf`: §3.4 specifies OnLand's runtime-coordinate source, inside-city
  short circuit, strict terrain/water comparison; EndTowing's reentrant
  pointer handling and releases; angular-proximity signature, normalization
  side effect and strict two-degree tolerance; direction-quantization signature
  and local-only normalization. §5.3.2 gives far-docking destination observations,
  entry effects, occupant 3D distance, strict distance boundary, time clamp,
  unconditional AI/movement/collision order and true return. Eight vectors.
- `6318e394`: §5.7.2 defines sinking roll/pitch axes and independent byte signs,
  reconstruction from previous heading, current-heading feedback, transform
  flags, publication order and degenerate-input limits. Four vectors distinguish
  sign choices and heading feedback from simply assigning visual forward.
- `f932ec06`: §9 specifies the borrowed shared debug string, cross-boat lifetime,
  argument/result contract, exact data sources and lack of explicit failure
  recovery. Two vectors. The final validation correction gives these the unique
  IDs WC3-DBG01/02; WC3-L01/02 already belonged to prior construction vectors.
- Final count: **138 unique named spec-side vectors**, 14 added. These are
  implementer requirements and documentation consistency checks, not executed
  engine tests or game measurements.
- Gate: `python tools/doccheck.py check`, questions index/check, vector/open-question
  validator and `git diff --check` pass. Initial vector validation found duplicate
  new debug IDs; corrected. Its subsequent count failure was a validator regex
  excluding existing numeric-only IDs; corrected and final count passed.
  Cargo/clippy/ctest: n/a for this spec-only lane.
- Evidence: read-only main-checkout export, decomp and PE constant reads; arithmetic
  and order checked against instruction bytes with address citations in the spec.
  No REA calls or failures. Ignored helper scripts live only in this worktree's
  build-runs; no RE output or scripts committed.
- Questions: WC3/WC4/WC5 remain in questions/watercraft.md with exact residuals;
  no new IDs filed or questions moved to answered. Index regenerated, unchanged.
- No engine access, plugin/capi/coverage-map edits, ABI export changes, proposed
  ABI rows, game requests or game runs. No shadow comparison applies.
- Reserved detached HEAD committed normally. Git printed shared packed-refs.lock
  permission diagnostics while commits succeeded; shared refs/config were untouched.
- Answer investigation stopped at measured context 96,004/150,000, within the
  two-thirds limit; the remaining budget was reserved for validation and reporting.

## Handoff

- Goal: complete remaining owner contracts without treating inventory counts as closure; specs only.
- Done: db7e11cf helpers/far docking; 6318e394 sinking attitude; f932ec06 debug lifetime. Final validation fixes duplicate debug IDs; 138 unique vectors, all doc gates pass.
- Next WC3: LoadDriveParameters 00567310 full signature/failure audit and inherited initialization/teardown plus subsidiary animation/update computations.
- Next WC4: Engine_AIBoatMotion 00571880 full selection, inherited player-drive, slots 83/103, effect-controller and slots 104/107. Far docking is now specified; its AI call is not permission to use an opaque provider.
- Next WC5: full grounding/heel/contact transform 0056AF90. Sinking 0056A440 basis/heading feedback now specified; do not redo footprint/shore-corner work.
- Evidence main: `C:/Users/tlori/FreeCity4/tools/re_query.py`, `decomp/cSC4Watercraft.c`, `decomp/_unnamed/region_00560000.c`.
- Fixed PE data: 00B0A154=2 degrees, 00B0A18C=1, 00B0A148=7. Sinking uses previous heading +300 for basis and transforms current heading +30C by it before normalizing.
- Scripts: ignored build-runs/read-pe.py and validate-spec.py. No game requests or REA operations outstanding.
- Detached HEAD; never update refs. No engine access or ABI changes. Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-watercraft-wc5c-r1-manifest.json`.

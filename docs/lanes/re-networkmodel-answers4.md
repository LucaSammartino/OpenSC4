| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-networkmodel-answers4 (NO21-NO23, NO25, NO26, network-occupants.md §10 model instance) | `re-networkmodel-answers4` / (RE spec lane) | done: NO21-NO23, NO25, NO26 answered DIFFERS in §10.1-§10.3; §2.2 +0x168/+0x16C fixed | engine: port §10.3 Clip (outside-anchored lerp) and §10.1 dtor; re/: rename 0x00604CA0 / 0x00647260 via re/symbols + merge_names.py in the main checkout |

## Report

**What changed.** The `docs/specs/network-occupants.md` §10 correction plus three new subsections:
- §10.1 says who builds the instances and what `GetModelInstances` returns.
- §10.2 gives the order of the passes.
- §10.3 describes the four bodies, with a table of the plane formulas and a worked example.

The question files and `docs/specs/QUESTIONS.md` are regenerated.

**REA.** It answered "No app is open", so all of this comes from static evidence: `re_query.py` asm and the
`decomp/` files for cSC4NetworkOccupantModelInstance, cSC4NetworkOccupantWithPreBuiltModel and
cS3DModelProcessor. Every number was checked against the instructions. The constants 0.5 (`0x00A84D2C`), 0.0625
(`0x00AA81E4`), 0.0 (`0x00A81054`) and the jump table `0x00614CE4` were read from the exe with
`build-runs/dd.py`.

**Findings.** All four answers DIFFER from the core-networkmodel4 defaults.
- **The four bodies are not overrides.** No vtable or data reference points at them. They are non-virtual
  members called only from the prebuilt `GetModelInstances` `0x0061E8A0`.
- **"Prelight" is misnamed.** `0x00614350` is a bind: `cS3DModelProcessor::SetModel` makes a private copy of the
  vertices.
- **Pass order:** Bind, Deform, FlipAndLight. With network flag `0x40000` it is Clip, Bind, FlipAndLight, with no
  Deform. Option `0x6A845768` skips the passes.
- **NO21 (Deform):** a two-triangle split, not bilinear.
  - The diagonal is v0–v2, or v1–v3 when flag `0x1000000` is set.
  - The planes come from `RecomputeShearTransform` `0x0061D230`.
  - The bend is added to the flat y, on a fresh copy every time.
  - The frame is centred, corners at ±8, set after the rotation table and the flip.
  - Nothing is clamped: the planes extrapolate.
- **NO22 (flip and lighting):** the flip mirrors by negating row 0 of the transform. FlipAndLight only reverses the
  winding (index lists and ranges), then calls `LightModel(*0x00B43DDC, flip)` and `ReleaseModel`.
- **NO23 (lifetime):** each instance is a separate 0xE4-byte heap object, built afresh on every prebuilt
  `GetModelInstances` call and cached at `+0x1C4`, with a second model's instance at `+0x1C8` (from `+0x19C`).
  The plain `GetModelInstances` `0x0061AF90` never returns null: it always emits the piece's embedded `+0x38`
  instance.
- **NO26 (release):** the prebuilt `Shutdown` `0x0061D0D0` releases both caches.

- **NO25 (Clip `0x00618B00`):** keeps lo.y <= y <= hi.y, so the call site keeps geometry at or above h. The
  cut vertex is lerped from the outside end: t = (plane - out)/(in - out) stored f32, a' = (in - out)*t + out
  (`0x00614490`/`0x00614170`). Format-10 colour bytes are truncated by `_ftol2`. The output is a fan and cut
  vertices are never shared (`0x006175C0`). Bounds are clamped to the box, not recomputed (`0x00618FD2`). An
  unsupported format leaves the mesh empty. Vector D in §10.3 gives the exact bits: outside-anchored 1.0
  (`0x3F800000`), inside weighted form 0.99999994 (`0x3F7FFFFF`), checked with an f32 simulation.
- **Destructor:** the body `0x00604CA0` releases +0x1C8 then +0x1C4, does not clear them, and tail-jumps to
  `0x0061C170`. §2.2: +0x168/+0x16C are f32 path-origin x/z (`0x0061B635`, `0x0061EEC9`), now fixed in the spec.

**Open.**
- NO24 and terrain TR33-TR37 were deliberately left alone.
- Names: `re/names_windows.tsv` still labels `0x00604CA0` as `cSC4NetworkOccupant::~cSC4NetworkOccupant` (it is
  the cSC4NetworkOccupantWithPreBuiltModel destructor body) and `0x00647260` as the prebuilt destructor (it is
  `cSC4NetworkTunnelOccupant::~cSC4NetworkTunnelOccupant`). The fix is a manual `re/symbols/*.tsv` row, but
  `tools/symport/merge_names.py` needs the gitignored `re/symport/win_features.pkl`, which this worktree does
  not have. So the rename was not made: the coordinator can add the rows and regenerate in the main checkout.

**Commits:** 04c1f969, 0f37aa05, 6bf75268, b8671525 (vector D, NO25 moved to answered, questions index).

No engine, tools or ABI changes, and no game run.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-networkmodel-answers4 (NO21-NO23, NO25, network-occupants.md §10 model instance) | `re-networkmodel-answers4` / (RE spec lane) | in progress (round 3): §10.3 Clip written from bytes, dtor + §2.2 +0x168/+0x16C fixed; NO25 still in the open file | RE: finish vector D, move NO25 to answered, questions.py index, merge-queue |

## Handoff (round 3, 2026-10-22)
- Goal: NO25 Clip 0x00618B00 (kept side, cut-edge interpolation), prebuilt dtor +0x1C4/+0x1C8, +0x164..+0x16C naming.
- Done (this commit): §10.3 Clip bullet rewritten as 9 numbered rules + worked vectors A-D. Facts: keeps lo.y <= y <= hi.y,
  so the call site keeps geometry at/above h. Y-slab clipper 0x006175C0 (24-byte 0x00618030). The cut vertex is lerped
  from the OUTSIDE vertex: t=(plane-out)/(in-out) stored f32, a'=(in-out)*t+out (0x00614490/0x00614170). Format-10
  colour bytes are trunc via _ftol2 (0x00614200). Output is a fan; cut vertices are never shared. Bounds are clamped
  to the box, not recomputed (0x00618FD2). An unsupported format leaves the mesh empty. §10.1: the dtor body
  0x00604CA0 (mislabelled cSC4NetworkOccupant::~) releases +0x1C8 then +0x1C4 and does not clear them, then runs
  base 0x0061C170. §2.2: +0x168/+0x16C are f32 path-origin x/z (GetPathInfo 0x0061B635, GetModelInstances
  0x0061EEC9). Slot 63 0x00604E10 reads +0x170/+0x174, and §2.4 was fixed to match.
- REA: "No app is open"; static fallback (re_query asm + decomp/_unnamed/region_00610000.c).
- Next (exact): (1) in spec §10.3 Clip, vector D still has the vague "other roundings" text. Replace it with: native,
  from the outside end: t=f32 0x3EAAAAAB and x'=3t -> 1.0. From the inside end: t'=f32 2/3 gives 0.99999994
  (0x3F7FFFFF), wrong. (2) Move NO25 from questions/network-occupants.md to answered/ as DIFFERS. (3) Run
  tools/questions.py index. (4) Lane report, merge-queue json (kind spec, abi false), return ready.
- Names note for re/: 0x00604CA0 is the prebuilt dtor body, 0x00647260 the tunnel dtor body. Not edited (docs only).

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

**Open.**
- NO25: which side `Clip` `0x00618B00` keeps, and how its cut edges interpolate.
- The prebuilt destructor's handling of the two caches was not examined.
- NO24 and terrain TR33-TR37 were deliberately left alone.

**Possible conflict for the spec owner.** The prebuilt code reads `+0x164` as a gate and `+0x168`/`+0x16C` as f32
origin offsets. §2.2 names those fields path-id and int64.

No engine, tools or ABI changes, and no game run.

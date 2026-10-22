# view-utilities: answered questions (archive)
<!-- Archive for [view-utilities.md](../../view-utilities.md). Lanes do not read this file. Open questions are in ../view-utilities.md. -->

<!-- from: re-viewutils (2026-10-20) -->
- ~~**VU1 - §7.1:** what mesh primitive types 6 and 7 are, and whether their odd triangles are intended: type 6
  gives (2i, 2i + 1, 2i + 2) for even i but (2i, 2i + 2, 2i + 3) for odd i, so a 4-vertex group reads vertices 4
  and 5; type 7 repeats triangles 1 and 2. A survey of the shipped S3D models' group types would settle whether
  either is ever picked. [the walk as measured]~~
  **ANSWERED 2026-10-20 (re-geometry):** graphics-formats.md §3.7: type 6 is a quad list and type 7 a quad strip;
  every shipped S3D primitive is type 0, so the odd walk is met only by meshes built at run time (the foundation
  faces, drawn directly, and `AddBoxMesh`, geometry-utilities.md §6.2, neither of them picked). The walk stays as
  measured; view-utilities.md §7.1 and §10 now say so.

<!-- from: core-view-utilities4 (2026-10-22) -->
- ~~**VU7 - §4.1 failed camera eye ray:** the viewport ground helper always returns true and ignores
  camera/ray failure; what are the temporary origin/direction contents and output in those cases?
  [safe Rust initializes the temporary ray to zero, then performs unguarded intersection, producing NaNs;
  direct ViewportPositionToRay failure preserves caller output; no guess at uninitialized native data]~~
  **ANSWERED 2026-10-22 (re-view-utilities-contracts4):** DIFFERS — §4.1: failed direct ray preserves caller outputs; ground wrapper uses uninitialized stack slots and returns true with indeterminate coordinates.

<!-- from: core-view-utilities4 (2026-10-22) -->
- ~~**VU9 - §§2,5,6 cache identity/source:** are +0x444/+0x445 the visible-rectangle and viewport-ground
  caches respectively? §5 says both caches clear on every draw rectangle yet full viewport ground
  points are computed only "if not cached". Is §6 always based on full viewport points, even while
  draw-frustum is narrowed? [literally clear both caches each draw and camera update; recompute full
  viewport ground points on draw; §6 uses full viewport points literally; no native cache-identity credit]~~
  **ANSWERED 2026-10-22 (re-view-utilities-contracts4):** DIFFERS — §6: +0x444 caches visible rectangles built from fresh full-viewport rays; +0x445 caches only whole-draw viewport corners; every draw clears both.

<!-- from: core-view-utilities4 (2026-10-22) -->
- ~~**VU11 - introductory related-spec paragraph / missing owned rotation method:**
  `RotateVectorByViewRotation` at 0x007F6CC0 is named as documented by lighting.md, but no body is given
  in view-utilities §§1–7,9,11 (this task's specified behavior source). Is it one of the attributed 49
  owner functions? Please include its full rotation/sign/output and invalid-input contract here or
  explicitly reconcile its ownership/source in the requested inventory. [unimplemented, no guessed
  alias to fixed-view projection; report 38 implemented named contracts, no whole-owner credit]~~
  **ANSWERED 2026-10-22 (re-view-utilities-contracts4):** DIFFERS — §4.3: free helper 0x007F6CC0 rotates in place by signed rotation%4, preserves y, and leaves negative nonzero remainders unchanged.

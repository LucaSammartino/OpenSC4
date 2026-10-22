# view-utilities: open questions
<!-- Questions about [view-utilities.md](../view-utilities.md). Append new entries at the END; when one is answered, move it to answered/view-utilities.md. Rules: README.md. -->

<!-- from: re-viewutils (2026-10-20) -->
- **VU2 - §1:** what the fields `+0x3DC`-`+0x410` hold; the constructor sets `+0x3F4` and `+0x3F8` to 1 and
  `+0x410` to 0.3, and none of the functions in this spec reads them. [unused here]

<!-- from: re-viewutils (2026-10-20) -->
- **VU3 - §3.1, §5:** whether the camera's view volume is always centred (left = -right, bottom = -top). The
  corners are built from the half-extents around the forward axis, so an off-centre volume would draw where
  the frustum tests do not look. [centred (inferred)]

<!-- from: core-view-utilities4 (2026-10-22) -->
- **VU4 - §§1–7 complete owner inventory/lifecycle:** the task attributes 49 functions/19670 bytes to
  SC4ViewUtilities, but the spec provides no exhaustive function inventory, setter/destructor/init bodies,
  or address-to-method ownership accounting. Please publish all 49 attributed entries with signatures,
  lifecycle/setter/getter bodies and any alias/helper ownership. Are any public methods absent here?
  [implement the named contracts and owned constructor state; Rust input setters are explicitly adapters;
  refuse complete-owner/19670-byte credit until the 49 functions are reconciled]

<!-- from: core-view-utilities4 (2026-10-22) -->
- **VU5 - §§2–4 numeric conventions and invalid camera input:** what exact table float bits, trig
  precision, intermediate evaluation order, inverse-scale calculation and vector normalization are used?
  What happens for zoom outside 0–4, zero viewport/volume extents, coincident eye/look-at, or vertical
  forward? [literal written f32 formulas, separate operations/no FMA; invalid zoom returns false/None;
  unguarded zero divisors retain IEEE NaNs/infinities; not a claim of bit-exact native float behavior]

<!-- from: core-view-utilities4 (2026-10-22) -->
- **VU6 - §§3.3,4.1,7 geometry dependency contracts:** specify RayIntersectsTriangle winding/culling,
  determinant tolerance, barycentric edge inclusion, t=0/negative-t treatment and whether distance is
  ray parameter or Euclidean length; RayIntersectsPlane parallel/t=0/failure-output behavior; polygon
  winding, clip-edge order and boundary/empty/degenerate float handling for ClipPolygonToPolygon2D.
  [own literal two-sided Moller–Trumbore, no epsilon, inclusive edges, t>=0, parameter t; exact parallel
  ray-plane miss preserves output; inclusive convex clipping x>=0,x<=W,z>=0,z<=D; unresolved fidelity]

<!-- from: core-view-utilities4 (2026-10-22) -->
- **VU7 - §4.1 failed camera eye ray:** the viewport ground helper always returns true and ignores
  camera/ray failure; what are the temporary origin/direction contents and output in those cases?
  [safe Rust initializes the temporary ray to zero, then performs unguarded intersection, producing NaNs;
  direct ViewportPositionToRay failure preserves caller output; no guess at uninitialized native data]

<!-- from: core-view-utilities4 (2026-10-22) -->
- **VU8 - §§4,4.2,5 integer conversions:** specify nearest rounding tie handling and out-of-range
  conversion for bounding rectangle coordinates; block<=0, nonfinite/out-of-int32 inputs and rounded-result
  overflow; rectangle centre expression/truncation and overflow. [existing x87::rhe ties-even helper for
  pixel bounds; block helpers truncate finite int32 input then mathematical floor/ceil, fail on invalid
  block/result; widened integer centre sum/2 truncates toward zero]

<!-- from: core-view-utilities4 (2026-10-22) -->
- **VU9 - §§2,5,6 cache identity/source:** are +0x444/+0x445 the visible-rectangle and viewport-ground
  caches respectively? §5 says both caches clear on every draw rectangle yet full viewport ground
  points are computed only "if not cached". Is §6 always based on full viewport points, even while
  draw-frustum is narrowed? [literally clear both caches each draw and camera update; recompute full
  viewport ground points on draw; §6 uses full viewport points literally; no native cache-identity credit]

<!-- from: core-view-utilities4 (2026-10-22) -->
- **VU10 - §7 resource edge contracts:** define undersized group counts for n-1/n-2, invalid index/
  vertex references, byte strides below 12, missing meshes and index addition overflow. Is the §7
  transform matrix applied by row dot products or column combinations? [typed owned resources; reject
  malformed byte input, saturate undersized triangle counts to zero, skip invalid triangles; row-dot
  matrix application literally from "rows"; preserve specified type6/7 indices even out of bounds]

<!-- from: core-view-utilities4 (2026-10-22) -->
- **VU11 - introductory related-spec paragraph / missing owned rotation method:**
  `RotateVectorByViewRotation` at 0x007F6CC0 is named as documented by lighting.md, but no body is given
  in view-utilities §§1–7,9,11 (this task's specified behavior source). Is it one of the attributed 49
  owner functions? Please include its full rotation/sign/output and invalid-input contract here or
  explicitly reconcile its ownership/source in the requested inventory. [unimplemented, no guessed
  alias to fixed-view projection; report 38 implemented named contracts, no whole-owner credit]

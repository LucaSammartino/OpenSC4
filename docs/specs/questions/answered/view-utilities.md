# view-utilities: answered questions (archive)
<!-- Archive for [view-utilities.md](../../view-utilities.md). Lanes do not read this file. Open questions are in ../view-utilities.md. -->

<!-- from: glue-viewutil-apply12 (2026-10-24) -->
- ~~**VU17 — private mesh fixture resource layout after VU16:** §13 closes the callable signatures and gives the complete iterator record, but §7.1 publishes only mesh vertex stride/count/data offsets (+04/+06/+08). What are the mesh size and offsets/types for vertex format, optional index buffer, group count and group array, including any nested buffer records, as consumed by MeshPick and iterator select-group? These are needed to create an accessible, isolated mesh fixture rather than guess pointer offsets or inspect live resource memory. [Implement local triangle picking and direct iterator vertex-copy/index-decode using the published iterator layout. Leave MeshPick, ModelPick and iterator select-group uncompared until a complete private resource fixture is specified.]~~
  **ANSWERED 2026-10-26 (re-vu30):** DIFFERS — §7.1.1: a 12-byte mesh holds V/I/G pointers at +00/+04/+08. Vertex stride/count/data are V-relative. G.begin/end at +04/+08 derive group count; optional I contains a pointer to uint16 data. Complete consumed layouts and indexed/nonindexed worked picking fixtures are published.

<!-- from: close-viewutil29 (2026-10-26), VU17 fixture failure -->
VU17 follow-up (answered above): treating §7.1's stride/count/data offsets as mesh-relative is not a safe minimal fixture: close-viewutil29-house-r4a faulted at 0x007F91DB reading 0xCCCCCCD2 when unprovided mesh words held canaries, before any mesh comparison completed. Please specify the mesh-to-vertex-buffer pointer relationship explicitly, in addition to group/index layout. The lane withdrew this unsupported fixture and leaves MeshPick complete=0; it does not infer a replacement layout from the fault.

<!-- from: glue-viewutil-apply12 (2026-10-24) -->
- ~~**VU18 — cached draw-frustum field offsets:** §1 gives the draw state only as the +228–+3D8 range, and §5 identifies flag +228, rectangle +22C and ground points +2E4. What are the exact offsets of its two boxes, corners, volume, planes and ground lines (including any unused gaps)? [The R-class adapter consumes only the fully published view fields. Leave IsBoundingBoxInDrawFrustum and the draw branch of GetFrustumAABBCityClipped uncompared; do not infer a fixed displacement from the view layout.]~~
  **ANSWERED 2026-10-26 (re-vu30):** DIFFERS — §5.1: boxes +23C/+254, corners +26C, volume +2CC, ground points +2E4, planes +314, reduced-plane triples +374, lines +3BC/+3CC; padding only +229–+22B. Two different view-to-draw displacements apply. Predicate and city-clipped-box fixtures give independent outputs.

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

<!-- from: re-viewutils (2026-10-20) -->
- ~~**VU2 - §1:** what the fields `+0x3DC`-`+0x410` hold; the constructor sets `+0x3F4` and `+0x3F8` to 1 and
  `+0x410` to 0.3, and none of the functions in this spec reads them. [unused here]~~
  **ANSWERED 2026-10-22 (re-view-utilities-contracts4):** MATCHES — §2 reserved-state contract: the block has only constructor defaults; none of the reconciled 50 family bodies reads it. No further source-level semantics are evidenced.

<!-- from: re-viewutils (2026-10-20) -->
- ~~**VU3 - §3.1, §5:** whether the camera's view volume is always centred (left = -right, bottom = -top). The
  corners are built from the half-extents around the forward axis, so an off-centre volume would draw where
  the frustum tests do not look. [centred (inferred)]~~
  **ANSWERED 2026-10-22 (re-view-utilities-contracts4):** DIFFERS — §3.1: half-extents build corners about the forward axis without enforcing centering, while projection and point tests consume the actual off-centre bounds.

<!-- from: core-view-utilities4 (2026-10-22) -->
- ~~**VU4 - §§1–7 complete owner inventory/lifecycle:** the task attributes 49 functions/19670 bytes to
  SC4ViewUtilities, but the spec provides no exhaustive function inventory, setter/destructor/init bodies,
  or address-to-method ownership accounting. Please publish all 49 attributed entries with signatures,
  lifecycle/setter/getter bodies and any alias/helper ownership. Are any public methods absent here?
  [implement the named contracts and owned constructor state; Rust input setters are explicitly adapters;
  refuse complete-owner/19670-byte credit until the 49 functions are reconciled]~~
  **ANSWERED 2026-10-22 (re-view-utilities-contracts4):** DIFFERS — §12 reconciles 48 attributed entries/19574 bytes plus two dedicated helpers/174 bytes: 50/19748, including the previously omitted 78-byte rotation helper. §2 specifies borrowed renderer wiring and absence of separately attributed view lifecycle/setter bodies.

<!-- from: core-view-utilities4 (2026-10-22) -->
- ~~**VU5 - §§2–4 numeric conventions and invalid camera input:** what exact table float bits, trig
  precision, intermediate evaluation order, inverse-scale calculation and vector normalization are used?
  What happens for zoom outside 0–4, zero viewport/volume extents, coincident eye/look-at, or vertical
  forward? [literal written f32 formulas, separate operations/no FMA; invalid zoom returns false/None;
  unguarded zero divisors retain IEEE NaNs/infinities; not a claim of bit-exact native float behavior]~~
  **ANSWERED 2026-10-22 (re-view-utilities-contracts4):** DIFFERS — §2.2 gives literal angle bits, x87 trig/extended intermediates and store boundaries, inverse-scale sources, unguarded normalization/divisions and unchecked zoom-table reads; no false/None invalid-zoom contract.

<!-- from: core-view-utilities4 (2026-10-22) -->
- ~~**VU6 - §§3.3,4.1,7 geometry dependency contracts:** specify RayIntersectsTriangle winding/culling,
  determinant tolerance, barycentric edge inclusion, t=0/negative-t treatment and whether distance is
  ray parameter or Euclidean length; RayIntersectsPlane parallel/t=0/failure-output behavior; polygon
  winding, clip-edge order and boundary/empty/degenerate float handling for ClipPolygonToPolygon2D.
  [own literal two-sided Moller–Trumbore, no epsilon, inclusive edges, t>=0, parameter t; exact parallel
  ray-plane miss preserves output; inclusive convex clipping x>=0,x<=W,z>=0,z<=D; unresolved fidelity]~~
  **ANSWERED 2026-10-22 (re-view-utilities-contracts4):** DIFFERS — §7.2 gives 0.01f tolerance, two-sided inclusive line-triangle hits without a t sign check, Euclidean picking distance, tolerant plane coplanarity/miss-zero writes, and ordered clipping with unchecked intersection failures.

<!-- from: core-view-utilities4 (2026-10-22) -->
- ~~**VU8 - §§4,4.2,5 integer conversions:** specify nearest rounding tie handling and out-of-range
  conversion for bounding rectangle coordinates; block<=0, nonfinite/out-of-int32 inputs and rounded-result
  overflow; rectangle centre expression/truncation and overflow. [existing x87::rhe ties-even helper for
  pixel bounds; block helpers truncate finite int32 input then mathematical floor/ceil, fail on invalid
  block/result; widened integer centre sum/2 truncates toward zero]~~
  **ANSWERED 2026-10-22 (re-view-utilities-contracts4):** DIFFERS — §4.4 specifies float-bit bias conversion, signed low-word truncation, wrapping block arithmetic, integer divide faults and wrapping rectangle-centre sums.

<!-- from: core-view-utilities4 (2026-10-22) -->
- ~~**VU10 - §7 resource edge contracts:** define undersized group counts for n-1/n-2, invalid index/
  vertex references, byte strides below 12, missing meshes and index addition overflow. Is the §7
  transform matrix applied by row dot products or column combinations? [typed owned resources; reject
  malformed byte input, saturate undersized triangle counts to zero, skip invalid triangles; row-dot
  matrix application literally from "rows"; preserve specified type6/7 indices even out of bounds]~~
  **ANSWERED 2026-10-22 (re-view-utilities-contracts4):** DIFFERS — §7.2 specifies raw signed counts, unchecked vertex/index/stride accesses and null mesh faults, model/mesh order and literal row-dot matrix application; checked rejection is an adapter policy.

<!-- from: core-view-utilities5 (2026-10-23) -->
- ~~**VU15 — block-rounding scalar input width and vector 18:** §§4.2/4.4/12 do not state the scalar parameter width of RoundDownToNearestBlockBoundary / RoundUpToNearestBlockBoundary. §11.18 requires Up(2147483647,64)=-2147483584, but 2147483647 is not representable as float32 (it rounds to 2147483648, producing a different low word). Are these scalar inputs float32, float64, or retained x87 values? If float32, should vector 18 instead specify the post-truncation integer stage, or a representable entry input? [Retain the existing f32 portable entry adapter without guessing a native signature. Test the prescribed wrapping integer stage with n=INT_MAX independently; full entry-point coverage of this part of vector 18 remains open.]~~
  **ANSWERED 2026-10-24 (re-viewutil-vu16):** MATCHES — §13.2 establishes four-byte float32 scalar inputs from the native loads. §11.18 now identifies INT_MAX as an internal integer-stage vector; §11.23 supplies representable native entry vectors, including float32 2147483648 giving −2147483648 for Up with block 64. No source-language return type or x87 control mode is inferred.

<!-- from: glue-viewutil-shadow11 (2026-10-24) -->
- ~~**VU16 — callable native ABI for same-process shadow:** §12 gives logical arguments and x87 scalar returns, but (apart from the explicit free cdecl rotation helper in §4.3) does not specify calling conventions, stack argument widths/order, receiver placement, or concrete scalar return widths. Please publish callable native declarations for the 50-body family, especially MinOfFourFloats/MaxOfFourFloats (four f32 or f64 arguments, cdecl or callee-pop, f32 or f64 return adapter), and supported image-address binding. Which private owner calls can safely run read-only on live state, and which require isolated storage/providers? [Do not guess native signatures. The opt-in shadow measures only the explicitly documented cdecl RotateVectorByViewRotation entry, using local vectors and live cached view inputs. The other 49 bodies, including both min/max helpers, remain uncompared pending this contract; no whole-owner verification claim.]~~
  **ANSWERED 2026-10-24 (re-viewutil-vu16):** MATCHES — §13 provides callable convention/stack/type/result declarations for all 50 entries, eight ST(0) capture contracts, exact examined-image identity/RVAs, and live-read versus private-owner/iterator/provider rules. Min/max are cdecl with four float32 inputs; float32 or float64 ST(0) capture is explicit caller policy. §§11.22–25 add ABI, numeric and isolation vectors. This closes the signature question, without claiming the remaining native comparisons have run or resolving VU12/VU13.

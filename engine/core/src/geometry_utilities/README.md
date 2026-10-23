# Geometry utilities

Source: `docs/specs/geometry-utilities.md` §§1–6.4, 10.1 and 11.
Core only: no ABI, plugin, native pointers, shadow, replacement or game claims.
Existing camera geometry is private and uses a different ray contract; it is
left unchanged. The existing foundation RNGs implement `DirectionRandom`.

## Public 37-body checklist

This enumerates the public owner inventory, not 37/37 implementation or native
coverage. **Bounded** means the stated portable domain only; **partial** retains
unproved arithmetic or foreign dependencies. No byte-count completion credit.

| Public # / body | Actual portable support and remaining limits |
|---|---|
| 01 RayIntersectsBoundingBox | Partial PC24 normal adapter; ordered rejection and false-output preservation; no extended resident comparison model |
| 02 PointInRectangle2D | Bounded half-open ordered comparisons, quiet NaN outside |
| 03 BoundingBoxesIntersect | Bounded quiet NaN/zero selection and output gates; try API excludes missing required maximum |
| 04 GetNormalToTriangle | Partial PC24 normal adapter; no PC64/store/exponent model |
| 05 PointAndVectorToPlaneEquation | Bounded checked pc24::plane, y/z/x association; legacy value adapter |
| 06 LengthOfLineSquared | Bounded checked pc24::squared_length, z/y/x association |
| 07 LengthOfLineSquared2D | Partial normal f32 adapter, y/x association |
| 08 RayIntersectsTriangle | Partial PC24 adapter, reciprocal store/order and unordered branches; no PC64 model |
| 09 PointDistanceToRay | Partial PC24 adapter, per-dot associations and residual sum; zero direction outside exact domain |
| 10 PointDistanceToRectangle | Partial PC24 adapter, low/high/within branch tree including unordered |
| 11 RayIntersectsPlane | Partial PC24 adapter, per-function sums and branch-specific output mutation; N3 PC64 UNRUN |
| 12 RaysIntersect2D | Partial PC24 adapter, direct division and null-output algorithm; no extended resident model |
| 13 LinesIntersect2D | Partial PC24 adapter, reciprocal then parameter store; unordered lower checks |
| 14 TwiceTriangleArea | Partial PC24 adapter; no extended resident/store model |
| 15 LengthOfLine | Bounded checked pc24::length, z/y/x association and root |
| 16 LengthOfLine2D | Partial normal f32 adapter |
| 17 ScaleVector2D | Partial PC24 adapter, factor before multiplication; unordered leaves words |
| 18 RotatePoint2D | Partial legacy host trig; native trig/store equivalence UNRUN |
| 19 ClipPolygonToEdge2D | Bounded signed/backed PC24 API, scratch reuse, gates; typed uninitialized-crossing exclusion |
| 20 ClipPolygonToPolygon2D | Bounded signed/backed API; initial copy, literal40 scratch, signed minimum and visible tails |
| 21 SetNormal | Field copies; no dirty mutation |
| 22 SetColour RGB | Field copies and alpha1; no dirty mutation |
| 23 SetColour RGBA | Field copies; no dirty mutation |
| 24 SetLightDirection | Field copies; no dirty mutation |
| 25 GetBoxCorners | Component selection, no renderer algorithm |
| 26 Mesh | Partial local dirty sequence; existing foreign-bounds stand-in unchanged, no foreign algorithm credit |
| 27 BeginPrimitive | Partial local append/dirty sequence; Rust allocation success model |
| 28 constructor | Zero/null local fields; no native destructor/allocator model |
| 29 AddVertex | Bounded checked PC24 lighting/magic bytes, u16 slot/count wrap, no dirty write; valid array required |
| 30 AddBoxMesh | Bounded face order and wrapping u32 inline closure; supplied backing/allocation success |
| 31 Reset | Partial local count/resize sequence, preserves dirty and bounds; foreign backing retention remains open |
| 32 GetTextureFormatForPixelCode | Exact unsigned dispatch/output gating; unused auxiliary input |
| 33 CreateTextureFromSurface | Bounded supported descriptor/backing, signed stride/offset, access/cleanup; GU7 auxiliary fields unspecified |
| 34 CreateCompressedTextureFromSurface | Bounded positive aligned backed BGRA, owned block packing/descriptor, free before end-access |
| 35 CreateTextureFromText | Bounded byte-NUL/raw primitive API, integer wrapping/terminating layout, refs/access/common epilogue; output conversion nearest-even |
| 36 RandomDirection2 | Partial existing host RNG/trig model; no new numeric-stage proof |
| 37 RandomDirection3 | Partial existing host RNG/trig model; no new numeric-stage proof |

Names are exposed in Rust snake case. Value-returning normal/plane/rotation
functions need no output pointer; boolean intersection functions accept
`Option<&mut ...>` with per-body output rules. The clipping module exposes
signed n/m/max/count independently of accessible output backing. Vec adapters
require the defined PC24 domain and cannot expose a negative count or tail.
`try_bounding_boxes_intersect` reports the required-maximum precondition only
on the accepted/non-null-minimum branch. Other bool/value adapters retain
documented numeric preconditions rather than claiming universal equivalence.

## Compatibility limits

- GU1 is the pre-existing question about actual null-output ray callers.
- GU2 remains partial: checked `pc24` is explicitly nearest-even/masked with
  normal/zero inputs and intermediates, excluding extended exponent, PC64,
  subnormal and ambient-control equivalence. Double lighting constants are
  rounded using integer dyadics, not a blanket f64 oracle or fitted bits.
- GU3 remains partial: foreign bounds, containers and destruction are unproved.
- GU4 is publicly answered; raw font/service/surface calls remain injected.
  Only font lookup failure is an early false; defined post-acquisition failures
  restore mode, write outputs, release surface if present, then release font.
  Upload runs in mode3. No texture reference is acquired/released.
  Native unchecked null/uninitialized/nonterminating domains return typed
  Unsupported, not an invented native false or rollback result. The packed
  UTF-8 convenience API is a qualified adapter, not a reference/access oracle.
- GU5 remains partial for foreign bounds/invalid memory; overlap selects the
  second tied/unordered bound. Signed clipping never substitutes a point for
  an uninitialized failed crossing. Rust checks accessible backing, not unsafe
  native overflow. A typed error can follow already-observed mutations.
- GU6 remains partial for lifecycle/foreign dependencies. The 37-body public
  inventory does not imply full-owner/runtime/default coverage.
- GU7 asks which auxiliary fields the plain upload descriptor establishes;
  those fields are represented as None rather than guessed.

Tests encode all nine published examples, plus asymmetric coordinates,
noncommuting rotation/translation, boundaries adjacent to epsilon, unbounded
hits, degenerate geometry, NaN overlap, output mutation rules, clipping
capacity/unsupported crossings, random draw counts, mesh face order/bounds/colour wrapping,
font mode restoration and independently specified DXT3 bytes.
The lane report records all 26 distinguishing vectors before comparison and
separately reports portable execution. No spec PROPOSED status is rewritten.

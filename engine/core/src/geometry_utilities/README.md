# Geometry utilities

Source: `docs/specs/geometry-utilities.md` §§1–6 and the nine §10 examples.
Core only: no ABI, plugin, native pointers, shadow, replacement or game claims.
Existing camera geometry is private and uses a different ray contract; it is
left unchanged. The existing foundation RNGs implement `DirectionRandom`.

## Published method checklist

| Owner / section | Implemented methods |
|---|---|
| SC4GeometryUtilities §2 | LengthOfLine, LengthOfLine2D, LengthOfLineSquared, LengthOfLineSquared2D, TwiceTriangleArea, PointDistanceToRay, PointDistanceToRectangle |
| SC4GeometryUtilities §3 | GetNormalToTriangle, PointAndVectorToPlaneEquation |
| SC4GeometryUtilities §4 | RayIntersectsBoundingBox, RayIntersectsTriangle, RayIntersectsPlane, RaysIntersect2D, LinesIntersect2D, BoundingBoxesIntersect, PointInRectangle2D |
| SC4GeometryUtilities §5 | ScaleVector2D, RotatePoint2D, ClipPolygonToEdge2D, ClipPolygonToPolygon2D |
| cS3DMeshBuilder §6.1 | constructor, Reset, SetLightDirection, SetNormal, SetColour (both overloads), BeginPrimitive, AddVertex, Mesh; recompute bounds helper |
| Box helpers §6.2 | corner maker, primitive closure, AddBoxMesh |
| nS3D §6.3 | RandomDirection2, RandomDirection3 |
| nS3D §6.4 | CreateTextureFromText, plain BGRA upload, DXT3 encoding |
| Lifecycle | Rust-owned optional arrays and pixel buffers are released by Drop; external font/surface/texture work goes through host traits |

Names are exposed in Rust snake case. Value-returning normal/plane/rotation
functions need no output pointer; boolean intersection functions accept
`Option<&mut ...>` and preserve the published null-output quirks. Clipping
returns the written polygon, whose length is the output count; it cannot
overwrite a caller buffer. Initial polygon copy is uncapped as specified.

## Compatibility limits

- GU1 is the pre-existing question about actual null-output ray callers.
- GU2: published algebra does not define each native x87 operation/store or
  constant width. Separate f32 operations model ordinary 24-bit arithmetic;
  extreme exponent and exact native transcendental claims are withheld.
- GU3: missing builder lifecycle details use literal, commented choices.
- GU4: text rendering is injected, including default-font lookup and the
  actual upload result. Native resource ownership/failure ordering is unbound.
- GU5: conditional extrema retain the first value on unordered/tied comparisons;
  safe clipping caps each scratch result at 40. Native overflow is not emulated.
- GU6: no transform/sort contract or exact 37-entry inventory is published.
  The table above covers every named behaviour; 37/37 and 7509-byte complete
  owner credit must await the inventory and resolution of contract gaps.

Tests encode all nine published examples, plus asymmetric coordinates,
noncommuting rotation/translation, boundaries adjacent to epsilon, unbounded
hits, degenerate geometry, NaN overlap, output mutation rules, clipping
capacity/fallback, random draw counts, mesh face order/bounds/colour wrapping,
font mode restoration and independently specified DXT3 bytes.

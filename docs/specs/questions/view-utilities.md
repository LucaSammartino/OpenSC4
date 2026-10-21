# view-utilities: open questions
<!-- Questions about [view-utilities.md](../view-utilities.md). Append new entries at the END; when one is answered, move it to answered/view-utilities.md. Rules: README.md. -->

<!-- from: re-viewutils (2026-10-20) -->
- **VU1 - §7.1:** what mesh primitive types 6 and 7 are, and whether their odd triangles are intended: type 6
  gives (2i, 2i + 1, 2i + 2) for even i but (2i, 2i + 2, 2i + 3) for odd i, so a 4-vertex group reads vertices 4
  and 5; type 7 repeats triangles 1 and 2. A survey of the shipped S3D models' group types would settle whether
  either is ever picked. [the walk as measured]

<!-- from: re-viewutils (2026-10-20) -->
- **VU2 - §1:** what the fields `+0x3DC`-`+0x410` hold; the constructor sets `+0x3F4` and `+0x3F8` to 1 and
  `+0x410` to 0.3, and none of the functions in this spec reads them. [unused here]

<!-- from: re-viewutils (2026-10-20) -->
- **VU3 - §3.1, §5:** whether the camera's view volume is always centred (left = -right, bottom = -top). The
  corners are built from the half-extents around the forward axis, so an off-centre volume would draw where
  the frustum tests do not look. [centred (inferred)]

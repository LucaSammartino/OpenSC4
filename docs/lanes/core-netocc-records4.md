| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-netocc-records4 (network occupants: NO15-NO17 records, transform, underground/pipe/subway records) | `core-netocc-records4` / (engine lane) | ready at ec875778: engine/core/src/network_occupants §2.3 base record v4-v8 by object offsets, transform read rules + 56-byte legacy load, underground v3 / pipe v3 / subway v2 records, §3.2 per NO17; no owner class complete (honest partial); no ABI, no glue, game=false; gate cargo 3810 passed / clippy 0 / ctest 63/63 PASS; questions NO18-NO20 | 1. Live object <-> record mapping for NetworkPiece (transform/quad/triangles/bbox fields on the piece). 2. Answers NO18-NO20. 3. §3.6 connection occupants record. 4. Pole §4, pylon §6, §8-§10. 5. Glue/shadow. |

## Report

**What changed** (engine/core/src/network_occupants only; no capi, plugin, CMake, ABI, tools edits):

- `xform.rs`: `Xform` gains the `+0` change counter. `read_into` follows NO15: reset to identity (counter +1)
  only when the live mask is non-zero, OR each present bit (counter +1 each), scale bit set only when the scale
  differs from 1.0 (NaN differs). `legacy_load_into`: the 56-byte v4 form (9+3+1 f32, u32 low byte -> counter,
  mask never reset). Writer emits the live mask unchanged.
- `record.rs` (rewritten by object offsets, NO16): `Vertex` (24-byte stream form x,y,z,u,v,R,G,B,A of the ARGB
  dword), one quad of 4 vertices, the four scalars `+0xFC/+0x105/+0x100/+0x104`, `CellInfo` (u16 2, u8 count,
  u8, u32, count x {u8,u32}; another version consumes only the u16 and the read continues misaligned),
  76-byte `Triangle`s with alpha forced on read, interleaved bbox, `convert_v5_flags` (§2.3 table + 0x80 rule)
  below v6 then `& 0xFFFF1FFF`, the v<7 two-float tail and `LoadEffects` (cell `(x,z)` -> property 0x4835C6AB
  `(z<<16)+x`, v<8 mark 0x4835C6AC), ref + int64 in every version, `finish_read_alpha` (the render-state step).
- `underground.rs`: `UndergroundRecord` (u16 3 + full v8 base + 13-field tail, 137 + 24/wall vertex),
  pipe (u16 3 + underground + u32 `+0x270`) and subway (u16 2 + underground) codecs, exact versions;
  `UndergroundPiece` now holds the quad, cell, four heights `+0x24C..+0x258`, `+0x260..+0x262`, class ID;
  `GetPosition` 4-vertex average (f32 partial sums x 0.25) with strict-< height minimum; `SetTerrainHeights`
  (stores 4 + host recompute); `SetVertexHeights` writes vertex y; `IsImaginaryConnection` per NO17 e (own bit
  31 clear -> false, no neighbour -> false, neighbour by class ID at cell + offset table); `apply_record` sets
  `+0x261=1/+0x262=0`, `mark_loaded_subclass` `+0x260=1`.

**Measured:** 13 tests in tests_record.rs (exact byte vectors: transform 13-byte and 53-byte examples, the
137-byte underground tail worked example, record field offsets; V5 conversion examples; versions 3/4/5/6/7/8,
pipe/subway version failures, failed base skips tail; GetPosition example (8, 2.5, 8)). Gate `qbuild all`:
cargo test 3810 passed 0 failed (main baseline 3803), clippy 0, fc4verify build 55/55, ctest 63/63 PASS.
No game runs (no glue in scope).

**Questions filed** (docs/specs/questions/network-occupants.md): NO18 pipe/subway Read after a failed
underground read; NO19 v<7 cell rounding precision; NO20 IsImaginaryConnection dir > 3. (QUESTIONS.md index
not edited: outside this lane's allowed files; next free is now NO21.)

**Honest coverage:** no owner class is fully implemented. Remaining: live NetworkPiece <-> record mapping for
transform/quad/triangles/bbox/cell info (NetworkPiece holds none of these yet); geometry, triangles building,
lighting, drawing; uv ratios / class info; GetZipperPieceType mapping; NO14; §3.6 connection occupants; pole,
pylon, §8-§10; glue and shadow. Not guessed: NO14, model offsets, wall-vector building.

# graphics-formats: answered questions (archive)
<!-- Archive for [graphics-formats.md](../../graphics-formats.md). Lanes do not read this file. Open questions are in ../graphics-formats.md. -->

## From the graphics formats spec (re-formats2 lane, 2026-10-14)

graphics-formats.md specifies FSH and S3D as the 1.1.641 exe loads them. These are what it could not settle from the
exe and the shipped data alone.

<!-- from: From the graphics formats spec (2026-10-14) -->
- ~~**GF1 — graphics-formats §2.6:** under DX7 (the default renderer) the FSH codes `0x64`, `0x65`, `0x6D`, `0x78`,
  `0x7B` and `0x7E` load through the factory but `TranslateGDFormatToSourceFormat` (`0x0088D0B0`) cannot create a
  texture for them. What does a model show then (untextured, white, previous texture, not drawn)? None is shipped
  on a model; plug-in textures may use them. [unknown; FreeCity4 decodes them and shows the texture, *[choice]*]~~
  **ANSWERED 2026-10-14** from the exe (DX7 stage set-up `0x00884320`, disable path `0x008844A6`, translation
  `0x0088D0B0`) and written into graphics-formats §2.6. **Drawn untextured.** The texture has no surface, so the driver
  disables the stage's colour and alpha operations and the model is drawn in its diffuse colour and alpha. It is not
  skipped, and the previous texture is not sampled. Under OpenGL five of the six codes upload; `0x78` fails there too.
  The bracket (decode and show) differs from DX7 but matches the OpenGL renderer for five codes; it stays *[choice]*.

<!-- from: From the graphics formats spec (2026-10-14) -->
- ~~**GF2 — §2.6, §2.11:** for `0x6D`, `0x7E` and `0x78` the factory asks for BGRA/BGR with the non-reversed packed
  types (4-4-4-4, 5-5-5-1, eGDType 11), which under OpenGL would read the channels in a different order from the
  Direct3D layouts (A4R4G4B4, A1R5G5B5, R5G6B5) that FSH tools write. Is there any renderer path where the game shows
  them correctly, and which layout should FreeCity4 follow? [the Direct3D layouts, *[choice]*]~~
  **ANSWERED 2026-10-14** from the exe (the format and type tables of §2.6, DX7 `0x0088D0B0`) and written into
  graphics-formats §2.11. **No renderer path shows them correctly.** DX7 cannot create them (GF1). OpenGL reads
  `0x6D`/`0x7E` with the channel order reversed and alpha in the low bits, and `0x78` fails under both. The bracket
  (the Direct3D layouts) stands *[choice]*.

<!-- from: From the graphics formats spec (2026-10-14) -->
- ~~**GF3 — §2.4, §2.11:** DXT1 with A = 0 (13,762 shipped entries) asks for "DXT1 RGB", but DX7 translates both DXT1
  formats to the same surface. Does DX7 then show 3-colour-mode index-3 texels as transparent (so alpha-tested
  materials cut them out)? An in-game check on a model with such texels would settle it. [yes, inferred from the
  translation; FreeCity4 decodes them transparent]~~
  **ANSWERED 2026-10-14** from the exe (`TranslateGDFormatToSourceFormat` `0x0088D0B0`: formats 7 and 8 both become
  source format 10, the one Direct3D DXT1 surface) and written into graphics-formats §2.11. **Yes.** A Direct3D DXT1
  surface always decodes 3-colour-mode index 3 as (0, 0, 0, 0), whatever A says. Materials with alpha test or blending
  drop those texels; a material with neither shows them black. The bracket (decode them transparent) matches. Not
  checked in game.

<!-- from: From the graphics formats spec (2026-10-14) -->
- ~~**GF4 — §2.13:** the 24 `0x7B` entries (64 × 64 and similar, identity grey-ramp `0x24` palettes) are in group
  `0x891B0E1A`, used by terrain brushes, water and underground code (`0x0074D730`, `0x00752240`, `0x0065D810`).
  Which of them reads these, and does it go through the texture path (which DX7 cannot create) or read the pixels
  from `cS3DShapeFile` directly? [inferred: CPU reads of intensity maps]~~
  **ANSWERED 2026-10-14** from the exe (`cSTETerrainView3D::InitializeCityEdgeLayerHeights` `0x00757300`,
  `cSTETerrain::ApplyBrushComponents` `0x0074D730` with resource interface `0x1AB50C3F`) and written into
  graphics-formats §2.13. They are read on the CPU, never through the texture path. The two 257 × 1 ramps (`0xCA5CC9C0`,
  `0xCA5CC9C1`) are read pixel by pixel into the city-edge layer height tables. The brushes sample their bitmaps on the
  CPU; that the 22 square entries are those brushes is inferred, not traced to the brush data. The bracket matches.

<!-- from: From the graphics formats spec (2026-10-14) -->
- ~~**GF5 — §3.8:** `SetTexFiltering` (`0x007D4280`) overrides the material's filters with (nearest, nearest) or
  (linear, linear-mipmap-linear) when one of two draw settings is on, and `0x007D41E0` can override wrap. Which
  settings are these, what are their defaults, and which is on in normal play? Without an override, 137,480 shipped
  1.5 textures have filters (0, 0), i.e. nearest. [unknown; probably the trilinear override is on by default]~~
  **ANSWERED 2026-10-14** from the exe (`SetTexFiltering` `0x007D4280`, wrap `0x007D41E0`, render-property
  registrations `0x007DF320`, `0x007DF400` and `0x007E0120`, DX7 filter states `0x00884363`–`0x008843BB`) and the shipped
  `Graphics Rules.sgr`, and written into graphics-formats §3.8. The settings are the boolean render properties
  `kForcePointSampling` (nearest, nearest) and `kForceBilinear` (linear, linear-mipmap-linear); wrap is
  `kForceTextureRepeat`. All three default to false and no shipped rule sets them. So in normal play none applies, and
  (0, 0) means nearest with no mipmaps. The bracket's guess (trilinear on by default) is wrong.

<!-- from: From the graphics formats spec (2026-10-14) -->
- ~~**GF6 — §3.5:** byte order of the 32-bit vertex colour element (only `0x80004101`, 2 shipped blocks in model
  `0xCB93E2B4`). [inferred: Direct3D colour, bytes B, G, R, A]~~
  **ANSWERED 2026-10-14** from the exe (DX7 vertex format `0x008891C0`, diffuse flag at `0x008891EA`; copy for
  `0x80004101` `0x0088C7D0`) and written into graphics-formats §3.5. DX7 copies the 32-bit colour unchanged into the
  vertex's Direct3D diffuse colour, so the bytes are B, G, R, A. The bracket matches. OpenGL not traced.

<!-- from: From the graphics formats spec (2026-10-14) -->
- ~~**GF7 — §4.2:** layouts of RKT4 (`0x27812824`) and RKT5 (`0x27812825`), read by `GetAllModelInstanceIDs`
  (`0x00496C90`, `0x00497120`) and `IsStaticBuilding` (`0x00583B10`), not by `GetModelInstanceID`. [out of scope
  here; for the building-rendering lane]~~
  **ANSWERED 2026-10-14** from the exe (`GetAllModelInstanceIDs` `0x00496C90`: zoom mask `0x00496D06`–`0x00496D25`,
  RKT4 walk `0x00496F2E`–`0x00497078`, RKT5 `0x0049707D`–`0x004970CB`; occupant form `0x00497120`;
  `IsStaticBuilding` `0x00583B10`) and a sweep of the shipped `.dat` files, and written into graphics-formats §4.2
  ("All the models of an occupant"). RKT4 is a list of records, each (selector, x, y, z, kind) followed by an
  RKT0–RKT3 value set of that kind (3 values for RKT0/RKT1, 30 for RKT2, 5 for RKT3). Every record whose selector
  equals the occupant's state gives one instance and an offset (x, y, z as signed 16.16 fixed point). An unknown
  kind or a short record ends the walk. RKT5 is RKT1 plus state × `0x10000`. Property `0x0ABFC024` is a zoom mask
  checked first, and the occupant's flag `0x80` switches all six keys to `0x278129xx`. `IsStaticBuilding` only
  tests that the keys are present. Shipped: RKT4 on 618 props, RKT5 on 5 flora; four streetlights carry a stray
  value that stops their `0x27812924` walk after one record. The bracket (out of scope) is replaced by this answer.

<!-- from: From the graphics formats spec (2026-10-14) -->
- ~~**GF8 — §4.2:** where the model detail global `0x00B216FC` (zoom remap table `0x00B094A4`) is set, and from which
  option. [unknown; 0 in the image]~~
  **ANSWERED 2026-10-14** from the exe (`cSC4ModelMaker::Init` `0x004948BE`–`0x004948DF`, message handler
  `0x00496466`–`0x00496495`, render-property getter slot 7 of vtable `0x00ABB6C0`, registration `0x007E2330`) and the
  shipped `Graphics Rules.sgr`, and written into graphics-formats §4.2. The global is the integer render property
  `kModelLODOffset` (22) clamped to 0–2. It is read at model-maker `Init` and again on message `0xC9F775BB`, which
  `cSC4App::ApplyVideoPreferences` posts. The option is Texture Quality: High 0, Medium 1, Low 2; the default is 0.

<!-- from: From the graphics formats spec (2026-10-14) -->
- ~~**GF9 — §2.8:** 167 shipped DXT1 entries store their mip levels without the 16-byte padding, and the game reads
  the smallest two or three levels from the wrong offsets (the last one past the pixel data). Reproduce, or read
  them as stored? [reproduce, zero-filling past the entry, *[choice]*]~~
  **ANSWERED 2026-10-14** from the exe (level table in `SerialRead` `0x00800860`; the levels are read from the
  `size`-byte buffer of `Open` `0x00808F00`) and a sweep of all 45,387 shipped entries, and written into
  graphics-formats §2.8. All 167 are in group `0x1ABE787D` and each is followed by a `0x70` name block. The last level
  always reads that block's first 8 bytes, and no read passes the record end. Reproduce, as the bracket says, but with
  the record's bytes rather than zeros: zero only past `size`, which no shipped entry reaches *[choice]*.

<!-- from: From the graphics formats spec (2026-10-14) -->
- ~~**GF10 — §3.9:** semantics of ANIM frame rate (30, 10, 3, 2, 1, 0 shipped), mode (always 3), flags and
  displacement (always 0), as the animated model instance uses them. [not traced; for the rendering spec]~~
  **ANSWERED 2026-10-14** from the exe (animated instance `0x007FD870`, `SetFrame` `0x007FDAF0`, `Step`
  `0x007FDB80`, `SetSpeed` `0x007FC170`, `Advance` `0x007FC1C0`, `SetAnimMode` `0x007FAFC0`; prop driver
  `0x005ECFB0`) and written into graphics-formats §3.9. The rate is frames per second (speed = rate × 0.001 per
  ms); rate 0 never advances. The file's mode, flags and displacement are never read. The instance has its own
  mode: 3 (default) loops, 2 ping-pongs, other values play once and hold; automata attach with mode 1. `Advance`
  steps at most one frame per call.

<!-- from: From the graphics formats spec (2026-10-14) -->
- ~~**GF11 — §3.10, §3.11:** consumers of PROP (`skeleton`/`suit` pairs on 58 people models) and REGP (`fx_…` names on
  1,810 models), and the meaning of REGP's seven floats. [inferred: effect attachment points, position and
  orientation; not traced]~~
  **ANSWERED 2026-10-14** from the exe (`cSC4ModelRegEffects` `0x005E8320`, `0x005E8810`, `0x005E6160`; fire crew
  `0x00548F10`, `0x00548BD0`; airplane `Init` `0x005017C0`) and a sweep of the shipped models, and written into
  graphics-formats §3.10 and §3.11. PROP has no consumer. REGP `fx_` names become visual effects on automata,
  helicopters, tanks and bridges (not on buildings, props or flora); `regpoint01` gives fire crews a point. There
  is one record per animation frame (C = F in all 3,025 entries). The data hold a position and a unit quaternion,
  but the exe reads the last four floats as an axis and an angle in radians. The bracket is right about the
  position, not about the orientation.

<!-- from: From the graphics formats spec (2026-10-14) -->
- ~~**GF12 — §2.4:** does anything outside the 3D texture path read the FSH centre fields (non-zero in 722 entries) or
  bits 12–27 of `+0x0C` (non-zero in 294)? [none found]~~
  **ANSWERED 2026-10-14** from the exe (entry-header decoder `0x00808D70` and its callers `0x008008EE`,
  `0x00808FBB`, `0x008090A0`) and written into graphics-formats §2.4. The decoder never copies the centre fields or bits
  12–27; it keeps code, width, height, the directory tag, the pixel pointer, A and M. No other code parses FSH. The
  bracket (none) matches.

## From the FSH and S3D readers (filed by the coordinator from the `formats3` lane's report, 2026-10-14)

About `docs/specs/graphics-formats.md` and `engine/formats` (`fsh`, `s3d`, `keys`). All §8 tests 1–43 pass; the
release sweep reproduces the §8.4 counts. Not done: the info-only open of §2.9; a DXT1 A = 0 opaque-black decoder
variant (GF3). The current answer is in brackets.

<!-- from: From the FSH and S3D readers (2026-10-14) -->
- ~~**FS1:** §2.8 says bytes "beyond the entry" read as zero, but not where an entry ends. [it ends at the first of: its
  attached block (`next`), another entry's header, or `size`; only positions at or after its pixel start count]~~
  **ANSWERED 2026-10-14** from the exe (level table `0x00800A60`–`0x00800AF5`: offsets only, no end test) and
  written into graphics-formats §2.8. The original has no entry end: it reads each level at its aligned offset from the
  `size`-byte buffer, whatever lies there. The bracket's end (attached block, next entry header or `size`, at or after
  the pixel start) is the implementer's choice for the acceptance test, and it reproduces the 167. But the bytes decoded
  past that end are the record's own, not zero (GF9).

<!-- from: From the FSH and S3D readers (2026-10-14) -->
- ~~**FS2:** Q2 rejects what does not fit "except the mip-padding case": how is that case told from a truncated entry?
  [if the levels fit packed back to back but the 16-byte-aligned layout overruns, zero-fill; if even the packed levels
  do not fit, reject. Shipped data: exactly 167 zero-filled, 0 rejected]~~
  **ANSWERED 2026-10-14** from the exe (`SerialRead` `0x00800860` checks no sizes) and a sweep of all 45,387 shipped
  entries, and written into graphics-formats §2.8. The original makes no such distinction: it accepts every layout and
  reads whatever follows. The bracket's test (packed fits, aligned overruns: accept; not even packed: reject) is the
  implementer's choice and stands. The sweep reproduces it independently: 167 accepted that way, 0 rejected. In every
  one of the 167 the aligned layout overruns by exactly 8 bytes.

<!-- from: From the FSH and S3D readers (2026-10-14) -->
- ~~**FS3:** A record with `count` = 0 has no frame 0. [rejected]~~
  **ANSWERED 2026-10-14** from the exe (`SerialRead` `0x008008D0`–`0x008008DB`: the count from the header, 0 →
  fail) and written into graphics-formats §2.7. A zero count fails the load before any entry is read, in both the full
  and the info-only open. The bracket (rejected) matches.

<!-- from: From the FSH and S3D readers (2026-10-14) -->
- ~~**FS4:** The original checks the `0x7B` palette on frame 0 only. [every `0x7B` frame needs a 256 × 1 palette with
  all 256 entries inside the record]~~
  **ANSWERED 2026-10-14** from the exe (`SerialRead` reads the palette info and calls `GetTypeAndFormat` for frame 0
  only, `0x008008DE`–`0x0080090D`; the frame loop compares headers only) and written into graphics-formats §2.6. Only
  frame 0 is checked, and the palette is never applied. The bracket (every frame needs one) is stricter than the
  original. Check frame 0 only, and decode a later frame with no usable palette as the grey the original uploads
  *[choice]*.

<!-- from: From the FSH and S3D readers (2026-10-14) -->
- ~~**FS5:** For DXT1, the field A could differ between frames. [each frame reports its own (format, type);
  `Fsh::format()` is frame 0's]~~
  **ANSWERED 2026-10-14** from the exe (`GetTypeAndFormat` called once, from `SerialRead` `0x0080090D`, with frame 0;
  `SetBindingImageData` reads the stored pair at `0x00809BA7`) and written into graphics-formats §2.10. Every frame uses
  frame 0's (format, type); a later frame's A is never read. The bracket (each frame its own pair) differs. Report frame
  0's pair for every frame. The decoded pixels are the same either way, since the decoder ignores A (§2.11).

<!-- from: From the FSH and S3D readers (2026-10-14) -->
- ~~**FS6:** Negative sign-extended dimensions. [the power-of-two test is on the 32-bit sign-extended height, so a
  negative height fails; max(W, m) is signed, so a negative width becomes m]~~
  **ANSWERED 2026-10-14** from the exe (`SerialRead` `0x0080094E`–`0x0080095A` power-of-two test on the 32-bit
  sign-extended height; `0x00800964`/`0x00800976` signed max) and written into graphics-formats §2.7. A negative height
  fails the test, and a negative width becomes m. The bracket matches.

<!-- from: From the FSH and S3D readers (2026-10-14) -->
- ~~**FS7:** A DXT width that is not a multiple of 4: the level size from the formula is smaller than the whole blocks.
  [the missing bytes read as zero, whole blocks are decoded, and the image is cropped]~~
  **ANSWERED 2026-10-14** from the exe (`nSGLDX7::cTextureManager::LoadTextureLevel` `0x0088ACD0`, `CopyToSurface`
  `0x0088A380`: a compressed surface is filled with its whole linear size from the level pointer) and written into
  graphics-formats §2.8. DX7 reads the whole blocks from the level's start, so the missing bytes are the record bytes
  that follow, not zeros. Decoding whole blocks and cropping stands; take the missing bytes from the record (zero only
  past `size`), as for GF9. OpenGL not traced.

<!-- from: From the FSH and S3D readers (2026-10-14) -->
- ~~**FS8:** The DXT5 alpha divisions (/7, /5) have no stated rounding. [rounded down, as for DXT1]~~
  **ANSWERED 2026-10-14**: no original behaviour; the original never decodes pixels on the CPU, and the GPU does
  the DXT interpolation. The implementer's choice (round down, as for DXT1) stands; noted in graphics-formats §2.11.
  No DXT5 entry is shipped.

<!-- from: From the FSH and S3D readers (2026-10-14) -->
- ~~**FS9:** A vertex format with stride below 12, or with no position, while the bounding box reads 3 floats per
  vertex. [stride < 12 is rejected; no position with stride ≥ 12 is accepted and its box uses the first 12 bytes]~~
  **ANSWERED 2026-10-14** from the exe (bounding box `0x00801040`: three floats at each vertex start, step S, no
  format test) and written into graphics-formats §3.4. The original boxes the first 12 bytes of each vertex whatever the
  format, so with S < 12 the last vertex reads past the vertex data. The bracket (reject S < 12; accept no position with
  S ≥ 12 and box its first 12 bytes) matches where the original is defined.

<!-- from: From the FSH and S3D readers (2026-10-14) -->
- ~~**FS10:** A legacy VERT stride S below the format's standard stride, or a legacy block with an unknown format. [both
  rejected]~~
  **ANSWERED 2026-10-14** from the exe (legacy VERT keeps S as read; `RenderMesh` `0x007D4A80` draws with the standard
  stride) and written into graphics-formats §3.4. The original loads both. With S below the standard stride the draw
  reads past the vertex data. An unknown legacy format is read and boxed with S but drawn with stride 0 and no known
  layout. Rejecting both (the bracket) stays the choice.

<!-- from: From the FSH and S3D readers (2026-10-14) -->
- ~~**FS11:** PROP is "sorted after reading", but the sort key is not given. [file order kept]~~
  **ANSWERED 2026-10-14** from the exe (PROP reader `0x00807EED`–`0x00808264`; sort `0x00806B40` compares the `s16` at
  +2, the frame) and a sweep of the shipped models, and written into graphics-formats §3.10. The key is the frame,
  signed and ascending, by `std::sort`. Equal frames keep file order up to 16 entries, not beyond. Every shipped entry is
  (−1, 0). The bracket (file order) matches the shipped data but not the original in general. Sort stably by frame
  *[choice]*.

<!-- from: From the FSH and S3D readers (2026-10-14) -->
- ~~**FS12:** REGP lower-casing: ASCII or locale-aware? Which encoding for byte names? [ASCII lower-casing; names
  decoded as UTF-8 when valid, else Latin-1]~~
  **ANSWERED 2026-10-14** from the exe (`cRZString::MakeLower` `0x0090EAC5`; `setlocale` from
  `cRZLanguageManager::SetCurrentLanguage` `0x00928021`) and a sweep of the shipped models, and written into
  graphics-formats §3.11. An all-ASCII name is lowered with `tolower`. Any other name goes UTF-8 → UTF-16 → `towlower`
  → UTF-8 in the game language's locale, so its non-ASCII letters are lowered too. All 3,025 shipped names are ASCII.
  The bracket matches for them. For a valid UTF-8 name, also lower non-ASCII letters *[choice]*.

<!-- from: From the FSH and S3D readers (2026-10-14) -->
- ~~**FS13:** §4.1 when the fallback texture itself does not load. [the fallback key is returned anyway]~~
  **ANSWERED 2026-10-14** from the exe (`GetModelFrame` `0x00806DF5`–`0x00806E02`: the fallback lookup's result is not
  tested) and written into graphics-formats §4.1. The slot is left with no binding and its ID set to 0, so it is never
  retried and draws without a texture *[inferred]*. The bracket (the fallback key returned anyway) differs. Resolve the
  slot to no texture *[choice]*.

<!-- from: From the FSH and S3D readers (2026-10-14) -->
- ~~**FS14:** RKT lookups with a zoom above 4, a detail above 2, a rotation above 3, or too few values. [no model
  (`None`); RKT0 ignores zoom and detail; the RKT1 addition wraps]~~
  **ANSWERED 2026-10-14** from the exe (`GetModelInstanceID` `0x00496830`–`0x00496937`, table `0x00B094A4`, RKT2
  index `0x005F8AB0` = 4z + r) and written into graphics-formats §4.2. The original checks nothing. A large zoom reads
  the next table row or the pointers after the table. Too few values read past the array. r is a signed remainder, so it
  is −3 to 3. Detail is clamped when it is set. The bracket (no model) stands for zoom and value count. Clamp the detail
  to 0–2 and take the rotation mod 4 rather than refusing *[choice]*. RKT0 and the RKT1 wrap match.

## From applying the graphics-format answers (filed by the coordinator from the `apply9c` lane's report, 2026-10-14)

<!-- from: From applying the graphics-format answers (2026-10-14) -->
- ~~**FS15:** FS4: a later `0x7B` frame that has its own valid 256 × 1 palette. Does the original use that palette,
  frame 0's, or upload grey? §2.6 covers only a frame with no usable palette. [its own palette; a palette that is
  missing, the wrong size, outside the record, or behind a chain that leaves the record counts as "no usable
  palette" → grey, and the record is still accepted]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS** (accepted as a choice): the original applies no palette to any frame. `SetCurrentFrameByIndex` stores each frame's own palette (`+0x10`), but its only reader (`0x00808DE0`) is called once, by the frame-0 check in `SerialRead` (`0x008008FA`). Every `0x7B` frame uploads its indices as luminance (grey). The bracket (own palette, else grey; record accepted) equals that for the shipped grey ramps. graphics-formats §2.6.

<!-- from: From applying the graphics-format answers (2026-10-14) -->
- ~~**FS16:** FS12: the original's `towlower` works on UTF-16 code units, so it probably does not lower characters
  outside the BMP. Should FreeCity4 match that? [no: every character is lowered by Unicode simple case mapping, as
  §3.11 says]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS** for non-BMP or invalid names only; the bracket is accepted. The original's UTF-8 decoder (`0x00910E18`) makes no surrogates: a 4-byte character becomes one unit, its code point mod 0x10000 (U+1F600 → U+F600), which `towlower` may lower and which is written back. The decoder also stops at the first invalid sequence, cutting the name (`Café` → `caf`). Shipped names are all ASCII. graphics-formats §3.11.

<!-- from: From applying the graphics-format answers (2026-10-14) -->
- ~~**FS17:** §2.13 does not record the 15 shipped frames, in multi-frame DXT1 records, whose own A field would give the
  other DXT1 format (they report frame 0's format since FS5; the pixels are the same). Should it? [no code depends
  on it]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES (no code depends on it); the count is recorded in the §2.13 survey table as the lane's measurement (15 frames, all in multi-frame records, given frame 0's format). graphics-formats §2.13.

## From applying the exemplar, graphics, protection, landfill and agent-order answers (filed by the coordinator from the `apply10b` lane's report, 2026-10-15)

apply10b applied EX34, EX35, EX38, EX39, GF7, PR17, PR18, LG21, LG23 and AO1 (merged `0f3ad91`, ABI 50) and cited
EX36, EX37, PR19, AO2, LG22, LG24-LG26, RG25-RG28, NB19 and PG16. EX39 is applied as the spec says: the C ABI slice
setters now always store an array, and the vector readers read a scalar as empty. In game the shadows are exact
(development 928/928, protection 288/288, landfill 72/72) and a replacement run that zones a new landfill reaches the
LG21 view route (rotation 0). GF13 and GF14 are reserved by graphics-formats §8.5, so these questions start at GF15.
The lane's choices are in brackets.

<!-- from: From applying the exemplar, graphics, protection, landfill and agent-order answers (2026-10-15) -->
- ~~**GF15:** GF7 (graphics-formats §4.2): is the zoom mask's bit number the zoom index as passed (0-4, before the
  detail remap), and is the mask read as the property's first integer value whatever its stored type? [yes to both; a
  mask with no value counts as 0; [literal reading] in `keys::occupant_models`]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES, with the stored-type rule made exact (graphics-formats.md §4.2, "Details of the read"). The bit is the zoom as passed, before the detail remap (`0x00496D16`–`0x00496D25`; shift count mod 32). The mask is read with the converting getter (`0x005FD480` → variant slot 25 `0x009176C8`): scalars converted by data-formats §5.1 (floats truncated), an array by the text round trip (first element), and count 0 or Empty gives 0. All 1,009 shipped masks are Uint32 scalars.

<!-- from: From applying the exemplar, graphics, protection, landfill and agent-order answers (2026-10-15) -->
- ~~**GF16:** GF7 (§4.2): for b+0..b+3 (RKT0-RKT3) and b+5 (RKT5), does `GetAllModelInstanceIDs` write T and G to its
  out pointers, as it does for a matching RKT4 record? [yes, v[0] and v[1]; [inferred]]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES. T and G are written for every form: RKT0 at `0x00496D80`/`0x00496D8A`, RKT2 and RKT3 at `0x00496EFC`–`0x00496F06`, and RKT1, RKT5 and matching RKT4 records through the rule helper `0x00496830` (`0x00496929`–`0x00496933`), each time v[0] and v[1].

<!-- from: From applying the exemplar, graphics, protection, landfill and agent-order answers (2026-10-15) -->
- ~~**GF17:** GF7 (§4.2): when a rule needs a value past the end of the whole property (too few values), or the zoom
  is above 4, what is appended? [nothing, neither an instance nor an offset, matching `model_key`'s "no model"; inside
  the array an RKT3 record at zoom 3-4 does read into the next record, as the quirk says]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** (choice accepted). The original checks nothing for RKT0–RKT3 and RKT5: it always appends one instance, plus offset (0, 0, 0) if asked (`0x004970EA`), reading past the property's array, or past the remap table for a zoom above 4. An in-range RKT1 base of 0 also appends a 0 instance (`0x00496E1E`–`0x00496E35`). Appending nothing in the out-of-range cases is a *[choice]*, since the original's values are garbage; shipped data never reaches them. Only RKT4 checks lengths.

<!-- from: From applying the exemplar, graphics, protection, landfill and agent-order answers (2026-10-15) -->
- ~~**GF18:** GF7 (§4.2): which stored types do the model keys and the zoom mask accept (the original's read
  conversion)? [the integer types up to 32 bits, converted; anything else counts as absent; shipped keys are Uint32
  arrays; [choice] in `keys::property_u32s`]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** (choice accepted). The keys are read with `vec<u32>` (`0x005FF510`): the raw 32-bit words of any stored type with no conversion; a scalar gives an empty vector, and RKT0–RKT3 and RKT5 then read through a null data pointer *[inferred: crash]*. The zoom mask alone uses the converting getter (GF15). Every shipped key is a Uint32 array and every mask a Uint32 scalar, so treating other key types as absent is a safe *[choice]*.

<!-- from: From lot creation, the plop test, the lot developer and the graphics formats (2026-10-15) -->
- ~~**GF13:** what does a visual effect do with a NaN orientation? An identity REGP record (zero axis) makes the
  model-effects update divide by zero, so `SetTransform` receives a NaN rotation (graphics-formats.md §3.11; 344
  shipped records). [unknown; *[choice]* FreeCity4 uses no rotation for a zero axis. Watching one of the 1,320
  `blue_blinking_light_slow` effects on a bridge in the original would settle it: drawn in place, drawn elsewhere or
  not drawn. The effect side (`cISC4VisualEffect` slot 8 and the effect renderer) is not specified]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** in part (graphics-formats.md §3.11, "What the effect does with it"). The effect's `SetTransform` (slot 8, `0x005BE880`) composes the incoming transform with its own placement in its transform node (`0x005BEC20`): the composed translation never uses the incoming rotation and stays exact (`0x005BEDF3`–`0x005BEEA9`), but the composed rotation and the reference point are NaN (`0x005BF307`–`0x005BF388`, even with the zeroed offset). *[inferred]* particles placed through the rotation are not drawn; the bracket's "no rotation" draws them at the right place. The choice is acceptable as a cleaner reading, since the in-game look is still the only way to settle what the player sees; it is marked *[choice]* and not *[compat]*.

<!-- from: From lot creation, the plop test, the lot developer and the graphics formats (2026-10-15) -->
- ~~**GF14:** the automata's per-tick animation driver. graphics-formats.md §3.9 traces the prop occupant's driver
  (animation tick `0x86AD10EE`, milliseconds × time dilation) and `SetAnimMode(1)` from `cSC4Automaton::AttachAnim`,
  but not what an automaton passes to `Advance` (slot 15) each tick, nor when it calls `SetFrame` (slot 10) or
  `SetSpeed` (slot 14). [inferred: the same tick message and dilation as props; nothing in saves depends on it.
  Settled by tracing the automaton's `DoAnimationTick` or equivalent and its calls through instance slots 10, 14
  and 15; the fire crew's `DoAnimationTick` (`0x00549CF1`) is the place to start]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** (graphics-formats.md §3.9, Automata). The automata manager calls each automaton's `DoAnimationTick(µs, ms)` from the same tick `0x86AD10EE` (`0x00506AFC`–`0x00506B14`, call `0x0050A89C`); the base step (`0x005089F0`) passes the tick's milliseconds with **no time dilation**, does nothing while paused (unless emergency-paused), calls `Advance(ms)` on each instance, sends the end-of-animation notification (slot 102 with 1, 0) when events are queued and the first instance is on its last frame, and adds ms to `+0x1C0`. Pedestrians (`0x00544C90`) call `SetSpeed(0.01)` before each `Advance` (10 frames per second). No per-tick `SetFrame`. Nothing in saves depends on it.

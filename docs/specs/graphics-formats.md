# Graphics formats as the original loads them: FSH and S3D — behaviour spec

Status: round 1 (M6), written from the Windows 1.1.641 executable (`bin/SimCity4.exe`, image base `0x00400000`) and
checked against the shipped data of SimCity 4 Deluxe (GOG install, read-only). Public modding descriptions of the
two formats were used for orientation only; every field below was confirmed in the exe or in the shipped data, and
§9 lists where the two disagree (the exe wins).

Tags as in data-formats.md: *[compat]* observable, must match; *[choice]* FreeCity4 decision; *[inferred]* not
proven; *[quirk]* original behaviour that looks unintended but is reproduced (or that a replacement must survive).

Shared material cited and not repeated here: DBPF packages, the DIR record and the QFS decoder (data-formats §2),
the resource manager (data-formats §3), resource keys and serial-stream primitives (foundations §2.2, §2.5). All
multi-byte values are little-endian. "Stream getter" means the persistent stream's typed getters; each one either
reads its full width or fails, and a failure fails the whole record.

---

## 1. Scope

| Item | Where |
|---|---|
| FSH (`SHPI`) container, entries, codes, mipmaps, attachments, how textures are created | §2 |
| S3D (`3DMD`) chunks, vertex formats, materials, animation, props, regions; how a frame is drawn | §3 |
| How a model names its textures; exemplar model keys | §4 |
| Where the exe does it | §5 |
| Interfaces and replaceability | §6 |
| Quirks and choices | §7 |
| Conformance tests | §8 |
| Community descriptions versus the exe | §9 |
| What `engine/formats` should add | §10 |
| Open questions (also in QUESTIONS.md, GF1–GF12) | §11 |

Survey of the shipped data (scripts in the session scratch folder; root `.dat`, language `.DAT`, `Sku_Data`,
`Plugins`; for each key the first index entry of its package, data-formats §2.3):

| Data | Result |
|---|---|
| FSH, type `0x7AB50E44` | 40,549 records (EP1 2,278; SimCity_2 15,237; SimCity_3 7,494; SimCity_4 9,796; SimCity_5 5,618; Living Mall 126); 36,177 stored QFS-compressed (DIR-listed). **All parse and all pass the factory's checks (§2.7); 0 failures.** 45,387 bitmap entries |
| FSH, type `0x7AB50E45` | none shipped (the factory refuses to create it anyway, §2.1) |
| S3D, type `0x5AD0E817` | 42,915 records (SimCity_1 41,075; EP1 1,820; Living Mall 20), all in group `0xBADB57F1`; 42,868 stored compressed. **All parse with the exe's grammar (§3); 0 failures**, no trailing bytes |
| Model textures | every one of the 155,822 non-zero material texture IDs exists as (`0x7AB50E44`, `0x1ABE787D`, ID); 2 IDs are 0 |

Detailed survey tables: FSH §2.13, S3D §3.14, keys §4.2.

---

## 2. FSH

### 2.1 Resource types and factory *[compat]*

- The resource manager registers factory class `0x3AB50E2A` for types `0x7AB50E44` and `0x7AB50E45`
  (registration at `0x00457F16` and `0x00457F2A`, resource manager slot `0x4C`).
- `CreateInstance` (`0x00800B80`) creates an object only for `0x7AB50E44`. A `0x7AB50E45` key therefore never
  loads, although `SerialRead` (`0x00800860`) accepts either type.
- `cS3DShapeFile` (`Open` `0x00808F00`, `OpenInfoOnly` `0x00808FD0`) is the only `SHPI` parser in the exe: the
  magic is compared byte by byte there and nowhere else. No other DBPF type is read as FSH.
- Records may be QFS-compressed at the DBPF level (DIR-listed); the resource layer decompresses them before the
  factory sees them (data-formats §2.6). There is no compression inside the FSH (§2.12).

### 2.2 Container header *[compat]*

```
offset size  field
0x00   4     magic       "SHPI" (53 48 50 49), compared exactly
0x04   u32   size        bytes the loader allocates and reads, header included
0x08   u32   count       number of directory entries
0x0C   4     directory ID ("G264", "G354", …): stored, never checked
0x10   8×count directory  (§2.3)
```

`Open` reads the 16-byte header, checks the magic, allocates `size` bytes, copies the header there and reads the
next `size − 16` bytes from the record. Consequences:

- A record shorter than `size` fails the read, so the load fails. Bytes after `size` are ignored.
- `size` is not checked against anything else. A `size` below 16 makes the header copy overrun the allocation
  before the read fails *[quirk]*; FreeCity4 rejects `size < 16 + 8×count` *[choice]*.
- Nothing later is bounds-checked against `size` (directory offsets, entry sizes, attachment offsets) *[quirk]*.
  FreeCity4 checks every offset and length against the record and rejects what does not fit *[choice]*, except
  for the mip-padding case of §2.8.

Worked example (a shipped header, 16 bytes): `53 48 50 49 40 02 00 00 01 00 00 00 47 32 36 34` is `SHPI`, size
`0x240` (576), one entry, directory ID `G264`. That is the fallback texture of §4.1.

### 2.3 Directory *[compat]*

`count` entries of 8 bytes follow the header: a 4-byte tag (never read by the game) and a `u32` offset from the
start of the record to the entry's bitmap header. Entry *i* is frame *i* (§2.7). Offsets are not checked, and need
not be in order or 16-aligned (all shipped ones are ascending).

### 2.4 Bitmap entry header *[compat]*

16 bytes at the directory offset, then the pixels. Decoded by `0x00808D70`:

```
offset size  field
0x00   u8    code         bitmap code (§2.6)
0x01   u24   next         offset from this header to the next attached block, 0 = none (§2.5)
0x04   s16   width        sign-extended
0x06   s16   height       sign-extended
0x08   u16   centre x     not read by the 3D path
0x0A   u16   centre y     not read by the 3D path
0x0C   u32   misc         bits 0–11: signed 12-bit value A (sign-extended); bits 12–27: unused;
                          bits 28–31: mip count M (levels = M + 1)
0x10         pixels       level 0 first (§2.8)
```

- **A** is used only by DXT1 (code `0x60`): A ≠ 0 selects the "DXT1 with 1-bit alpha" texture format, A = 0 plain
  DXT1 (`0x00800690`, §2.10). It is stored as the low 16 bits of the sign-extended value, so any non-zero 12-bit
  value counts. For every other code A is ignored.
- Bits 12–27 and the centre fields are ignored by the 3D path. Other subsystems may read the centre fields through
  their own code; nothing found does *[inferred]*.

### 2.5 Attached blocks and palettes *[compat]*

`next` chains further blocks after the pixels. Each block starts with the same 16-byte header shape (its own code
byte and `next`). The game follows the chain only for codes `0x7B` and `0x7A`, and only to find a palette
(`SetCurrentFrameByIndex` `0x00808CE0`):

- It walks block to block (each `next` relative to the current block) until it meets a block whose code is
  `0x2A`, `0x24`, `0x2D` or `0x29`, or a block with `next = 0` (then: no palette).
- The palette's info is: code, width = `u16` at +4, height = `u16` at +6 (both **zero**-extended, unlike the bitmap),
  the `u16` at +10, and its data at +16 (`0x00808DE0`). Only width and height are ever checked (§2.6).
- Every other attachment (names `0x70`, comment-like blocks `0x6F`, palettes after other codes, anything else) is
  ignored by the game.

### 2.6 Bitmap codes *[compat]*

`GetTypeAndFormat` (`0x00800670`) maps the code to a renderer texture format and component type; the code's
`PixelWidthMul2` (`0x00808E40`, twice the bytes per pixel) sizes the levels. The enums are the renderer's
(`eGDTextureFormat`, `eGDType`, decoded from the OpenGL driver's tables at `0x00B120E8` and `0x00B12010`).

| Code | Pixel data | 2×bytes/px | Format, type | DX7 (default renderer) | Shipped entries |
|---|---|---|---|---|---|
| `0x60` | DXT1, 8 bytes per 4×4 block | 1 | A = 0: 7 DXT1-RGB; A ≠ 0: 8 DXT1-RGBA; type 0 | uploads (both formats become the same DXT1 surface, `0x0088D0B0`) | 19,357 |
| `0x61` | DXT3, 16 bytes per block | 2 | 9 DXT3, type 0 | uploads | 25,957 |
| `0x62` | DXT5, 16 bytes per block | 2 | 10 DXT5, type 0 | uploads | 0 |
| `0x64` | 8-bit alpha | 2 | 4 ALPHA, type 0 (signed BYTE) | **fails** | 0 |
| `0x65` | 8-bit luminance + 8-bit alpha | 4 | 6 LUMINANCE_ALPHA, type 0 (signed BYTE) | **fails** | 0 |
| `0x6D` | 16-bit 4:4:4:4 | 4 | 3 BGRA, type 8 (USHORT 4-4-4-4) | **fails** | 0 |
| `0x78` | 16-bit 5:6:5 | 4 | 2 BGR, type 11 (packed 16-bit, not mapped by the GL table either) | **fails** | 0 |
| `0x7B` | 8-bit index; requires a palette of exactly 256 × 1 (§2.5) | 2 | 5 LUMINANCE, type 0 (signed BYTE): the indices themselves, the palette is never applied | **fails** | 24 |
| `0x7D` | 32-bit, bytes B, G, R, A | 8 | 3 BGRA, type 1 (UBYTE) | uploads | 40 |
| `0x7E` | 16-bit 1:5:5:5 | 4 | 3 BGRA, type 9 (USHORT 5-5-5-1) | **fails** | 0 |
| `0x7F` | 24-bit, bytes B, G, R | 6 | 2 BGR, type 1 (UBYTE) | uploads | 9 |
| `0x7A` | (4-bit index) | 1 | — | rejected by the factory | 0 |
| any other | | | — | rejected by the factory | 0 |

Notes:

- "Fails" means the DX7 driver's `TranslateGDFormatToSourceFormat` (`0x0088D0B0`) has no entry for the
  (format, type) pair: it accepts only BYTE/UBYTE with RGB, RGBA, BGR, BGRA and the three DXT formats, plus three
  packed 16-bit types (eGDType 10, 13, 14) that FSH never produces. The factory still accepts these codes
  (the record loads), but the DX7 texture gets no surface.
- **A model whose texture has no surface is drawn untextured** (GF1). When the DX7 driver sets up a texture stage
  and finds no texture record, or a record whose surface pointer (+0x2C) is null (`0x00884320`), it sets that
  stage's colour and alpha operations to *disable* (`0x008844A6`) and binds nothing. Stage 0 then passes the diffuse
  colour and alpha (vertex colour and lighting) straight through: the model is still drawn, with its material's
  depth, blending and alpha-test states, in its diffuse colour. The texture bound before stays bound but is not
  sampled. Under OpenGL, `0x64`, `0x65`, `0x7B`, `0x6D` and `0x7E` do upload (signed bytes for the first three, the
  channel order of §2.11 for the last two); `0x78` fails there too, as type 11 has no GL entry. FreeCity4 decodes
  and shows all six *[choice]*: unlike DX7, the default renderer, but like the original's OpenGL renderer for five
  of them.
- Codes `0x64`, `0x65` and `0x7B` use type 0, which the OpenGL table maps to signed `GL_BYTE`: values 128–255 would
  clamp to 0 under GL *[quirk]*. FreeCity4 reads them unsigned *[choice]*.
- A code with bit 7 set (a "compressed bitmap" code `0xE0`–`0xFF`) is outside `0x60`–`0x7F` and rejected.
- For `0x7B` the palette check reads the palette info filled by §2.5. When no palette was found the info is not
  written, and the check reads whatever the caller's stack held *[quirk]*. FreeCity4 rejects `0x7B` without a
  256 × 1 palette *[choice]*. Only frame 0's palette is checked: the format is decided once, from frame 0 (§2.10),
  and later frames need no palette, which is never applied anyway (FS4). FreeCity4 checks frame 0 the same way and
  decodes a later frame with no usable palette as grey, (i, i, i, 255) for index i, which is the original's
  luminance upload read unsigned *[choice]*.

### 2.7 What the factory accepts (`SerialRead` `0x00800860`) *[compat]*

A record with `count` = 0 fails at once, before any entry is read (`0x008008D0`–`0x008008DB`), in the full and
the info-only open alike (FS3). Otherwise, with frame 0's header (code C, width W, height H, mip count M):

1. The code must map (§2.6), else the load fails.
2. **H must be a power of two** (the test is "H & −H equals H", so 0 passes). **W is not checked**: the shipped
   data has two 257 × 1 entries, and they load.
3. The minimum level dimension is m = 4 for DXT codes (`0x60`–`0x62`), else 1. The texture's dimensions are
   max(W, m) × max(H, m); the level count is M + 1. W and H are the sign-extended values and both tests are
   signed (FS6): a negative H fails rule 2 (H = −4: −H & H = 4, not −4), and a negative W becomes m.
4. Every further entry (frame) must equal frame 0 in **height, code and M**, else the load fails. Width is not
   compared; all frames are laid out with frame 0's dimensions (§2.8), so a narrower later frame is misread
   *[quirk]* (none shipped).
5. Nothing is checked against the record size (§2.2).

The result is one texture with `count` frames (animated or multi-frame textures), each with M + 1 levels.

### 2.8 Mipmap levels *[compat]*

For each frame, level offsets are relative to that entry's pixel start (header + 16):

- Level 0 starts at offset 0. Its working dimensions are w = max(W, m) and h = max(H, m), and its size is
  PixelWidthMul2(C) × w × h, halved (integer).
- Each next level starts at the previous start plus the previous size, rounded **up** to a multiple of 16.
- Going to the next level, w is halved only while it is above m, and so is h; each halving also halves the size.

So every level starts **16-byte aligned** from the pixel start, and DXT levels stop shrinking at 4 × 4 (8 or 16
bytes). Example: DXT1 8 × 8, M = 3: offsets 0, 32, 48, 64 (sizes 32, 8, 8, 8; padded total 80, unpadded 56).

A DXT width that is not a multiple of 4 (none shipped) makes the formula smaller than the whole blocks: DXT1 6 × 8
gives 24 bytes for 2 × 2 blocks of 8. DX7 copies the surface's whole compressed size from the level's start
(`CopyToSurface` `0x0088A380`, linear-size path), so the missing bytes are the record bytes that follow, padding first
(FS7; OpenGL not traced). FreeCity4 reads the whole blocks from the record the same way, decodes them and crops to
the width *[choice]*.

*[quirk]* 167 shipped DXT1 entries with mipmaps store their levels **without** the per-level padding (tools
packed them back to back). The game still reads them at the aligned offsets. Only levels after the first 8-byte
level move, so the two or three smallest levels show the wrong blocks, and the last level reads up to 8 bytes past
the pixel data (into the next attachment). Example: `SimCity_2.dat` (`0x7AB50E44`, `0x1ABE787D`, `0x10EC0000`),
128 × 128, M = 7: the game reads levels at 0, 8192, 10240, 10752, 10880, 10912, 10928, 10944; the data ends at
10944.

All 167 are in group `0x1ABE787D`, and each is followed by its `0x70` name block. The misread last level always lies
exactly 8 bytes past the stored levels, on that block's header: the game reads the code `0x70`, the block's 3-byte
`next` and the first four name characters as the level's DXT1 block. No shipped level read reaches past the record
(`size`). In the example the last level reads `70 00 00 00 31 30 45 43`: c0 = `0x0070`, (0, 12, 132) after
expansion, c1 = black, so a 4 × 4 level of dark blues and black. FreeCity4 reads the aligned offsets from the
record as the game does, so those bytes are the record's own; only bytes past `size` (none shipped) read as zero
*[choice]* (GF9).

The original has no notion of where an entry ends (FS1): it reads every level at its aligned offset from the
`size`-byte buffer, whatever lies there, and checks nothing. FreeCity4 needs an end only to decide acceptance
(below): an entry ends at the first of its attached block (`next`), another entry's header, or `size`, counting
only positions at or after its pixel start *[choice]*. The bytes it decodes still come from the record, as above.

Acceptance (FS2). The original accepts any layout (§2.7): a truncated entry shows the bytes that follow it, and a
read past `size` would show heap bytes. FreeCity4 accepts an entry whose levels fit before its end at the aligned
offsets, or whose levels fit there packed back to back while the aligned layout overruns (the mip-padding case,
read from the record as above); it rejects an entry whose levels do not fit even packed *[choice]*. Shipped: 167
entries of the second kind, none of the third. In all 167 the aligned layout overruns the end by exactly 8 bytes,
and the entry holds the packed levels, in 163 of them followed by 8 bytes of padding.

### 2.9 Info-only open *[compat]*

When the factory is asked for info only, `OpenInfoOnly` reads the header plus `roundup16(8 × count) + 16` bytes
(directory and the first entry header if it starts there) and fails if directory entry 0's offset is above
`roundup16(8 × count) + 16`. Frame consistency (§2.7 rule 4) is not checked in this mode, and no level table is
built.

### 2.10 Texture creation *[compat]*

- The texture binding factory (class `0x1AC0E11A`, interface `0xFAC0E219`) creates one renderer texture per frame
  (`SetBindingImageData` `0x00809B90`). It uploads the **last n levels** of the frame's level table, level k of
  the binding getting width `bw >> k` and height `bh >> k` and the table entry `(M + 1 − n) + k`. A binding
  smaller than the FSH (the LOD offset, `SetLodOffset` `0x00809910`, set from the texture-quality option
  *[inferred]*) therefore skips the largest levels.
- Every frame is created with the one (format, type) pair decided from frame 0: `GetTypeAndFormat` is called only
  from `SerialRead` (`0x0080090D`), and the upload reads the stored pair (`0x00809BA7`). A later DXT1 frame whose A
  differs still gets frame 0's format (FS5); FreeCity4 reports frame 0's pair for every frame *[compat]*.
- DX7 creates the surface through `TranslateGDFormatToSourceFormat` (§2.6). OpenGL uploads the DXT formats as
  compressed images and the others through its format and type tables.
- Bindings are cached by (group, instance): the hash at `0x0080A380` ignores the key's type.

### 2.11 Pixel decoding for FreeCity4 *[choice]*

The original never decodes pixels on the CPU for 3D textures (the GPU does), so nothing in the exe fixes the rounding
of the DXT interpolations: FreeCity4 rounds every division down *[choice]* (FS8). A standalone renderer that decodes
to RGBA8 uses:

| Code | Decoding |
|---|---|
| `0x60` | S3TC DXT1. Colours 5:6:5 expanded by bit replication. If c0 > c1 (as `u16`): c2 = (2c0 + c1) / 3, c3 = (c0 + 2c1) / 3; else c2 = (c0 + c1) / 2 and c3 = **transparent black**, whatever A says (DX7 behaviour: both DXT1 formats become the one Direct3D DXT1 surface, `0x0088D0B0`, which always decodes 3-colour-mode index 3 as (0, 0, 0, 0); under OpenGL A = 0 gives opaque black. A material with alpha test or blending drops these texels, one with neither shows them black: GF3). Divisions on the 8-bit expanded values, rounded down |
| `0x61` | DXT3: 8 bytes of 4-bit alpha (pixel 0 in the low nibble of byte 0, row-major), alpha × 17; then a colour block always in 4-colour mode |
| `0x62` | DXT5: a0, a1, 48 bits of 3-bit indices (pixel 0 in the lowest bits). Index 0 → a0, 1 → a1. If a0 > a1, index i = 2–7 → ((8 − i) a0 + (i − 1) a1) / 7; else i = 2–5 → ((6 − i) a0 + (i − 1) a1) / 5, 6 → 0, 7 → 255. Colour block in 4-colour mode |
| `0x7D` | B, G, R, A bytes |
| `0x7F` | B, G, R bytes, alpha 255 |
| `0x6D` | `u16` A4 R4 G4 B4 (A in bits 12–15), each × 17 |
| `0x7E` | `u16` A1 R5 G5 B5 (A in bit 15), 5-bit channels by bit replication, alpha 0 or 255 |
| `0x78` | `u16` R5 G6 B5, alpha 255 |
| `0x64` | (255, 255, 255, a) |
| `0x65` | byte 0 luminance L, byte 1 alpha: (L, L, L, a) |
| `0x7B` | index through the palette (0x24: B, G, R bytes; 0x2A: B, G, R, A; 0x2D: `u16` A1R5G5B5; 0x29: `u16` R5G6B5), alpha 255 unless the palette has alpha. The shipped palettes are identity grey ramps, so this equals the original's luminance upload |

The 16-bit layouts are the Direct3D ones that FSH tools write. **No renderer of the original shows them that way**
(GF2). DX7 cannot create them (§2.6). OpenGL uploads `0x6D` and `0x7E` as BGRA with non-reversed packed types, so it
reads a `u16` as B in bits 12–15, G 8–11, R 4–7 and A 0–3 (`0x6D`), or B 11–15, G 6–10, R 1–5 and A bit 0 (`0x7E`);
`0x78` does not upload under OpenGL either. Example: the `0x7E` texel `0xFC00` is opaque red in the Direct3D layout,
and OpenGL reads it as B 31, G 16, R 0, A 0, a transparent blue. FreeCity4 follows the Direct3D layouts *[choice]*:
they are what the files mean, and the original has no correct path to copy.

### 2.12 Compression *[compat]*

FSH records are compressed only by the DBPF layer (DIR + QFS, data-formats §2.6–§2.7): 36,177 of 40,549 shipped
records are. A QFS-compressed FSH body that is not DIR-listed reaches the factory as stored bytes, fails the magic
check and does not load. There is no in-file compression: codes with bit 7 set are rejected (§2.6).

### 2.13 FSH survey

| Measure | Result |
|---|---|
| Records / entries | 40,549 / 45,387; every `size` field equals the record length; every directory ascending |
| Groups | `0x1ABE787D` 36,514 (model and UI textures); `0x0986135E` 1,453 (lot base textures, `0x005DEE00`); `0x2BC2759A` 1,316 (network shadows, `0x0061E340`); `0x49A593E7` 851; `0x2A2458F9` 233; `0x891B0E1A` 182 (terrain brushes, water, underground: `0x0074D730`, `0x00752240`, `0x0065D810`) |
| Directory IDs | `G354` 22,895; `G264` 17,363; `G315` 209; `G290` 37; `G231` 19; `G352` 19; `G349` 3; `G341` 3; `G357` 1 |
| Entries per record | 1: 39,598; 2: 315; 4: 281; 3: 130; 5: 37; … up to 69 |
| Codes (entries) | `0x61` 25,957; `0x60` 19,357; `0x7D` 40; `0x7B` 24; `0x7F` 9. No other code |
| Sizes | `0x61` 4–256 × 4–512; `0x60` 2–256 × 2–256; `0x7B` 32–257 × 1–128; `0x7D` 2–256 square; `0x7F` 1–16 × 1–128. Most common: 256 × 256 (25,943), 128 × 128 (3,391), 32 × 32 (3,208), 64 × 64 (3,178). Non-square: 4,188. Non-power-of-two: 2 widths (257, code `0x7B`), no heights |
| Mip count M | 0 in 44,974 entries; DXT1: 4–8 (304 entries); DXT3: 2–8 (109); never on other codes |
| A (12-bit field) on DXT1 | 0: 13,762; 1: 5,030; 8: 563; −30: 2 |
| Bits 12–27 of +12 non-zero | 294 (ignored); centre fields non-zero: 722 (ignored) |
| Attachments | `0x70` (name text, e.g. a source path) 15,899; `0x6F` 102; palette `0x24` 24 (after every `0x7B`, 256 × 1, identity grey ramp); palette `0x2A` 3 (after `0x61` entries, ignored) |
| Level layout | 16-byte padded as §2.8 everywhere except 167 DXT1 mipped entries packed without padding |
| Frame checks | every multi-entry record has equal height, code, M and width across frames |
| Failures | 0 (all accepted by §2.7) |

The 24 `0x7B` entries live in group `0x891B0E1A` and cannot be created as textures under DX7; their consumers
(terrain brushes, water) are GF4.

---

## 3. S3D

### 3.1 Resource type and factory *[compat]*

Factory class `0xFAD0F0B6` is registered for type `0x5AD0E817` (`0x00457F66`). `CreateInstance` (`0x008065C0`)
accepts only that type and makes a `cS3DResModel`; `SerialRead` (`0x00806FA0`) checks the type again and parses the
record with the stream getters. All shipped models are in group `0xBADB57F1`.

### 3.2 Chunk sequence *[compat]*

A model is a fixed sequence of chunks. Each starts with a 4-byte tag, compared exactly (`strncmp`, 4 bytes), and a
`u32` length:

```
"3DMD" u32   then, in this order:
"HEAD" u32   §3.3
"VERT" u32   §3.4
"INDX" u32   §3.6
"PRIM" u32   §3.7
"MATS" u32   §3.8
"ANIM" u32   §3.9
"PROP" u32   §3.10, only in versions 1.1 and later
"REGP" u32   §3.11, only in versions 1.2 and later
```

- **Every length field is read and ignored** *[compat]*, including the one after `3DMD`. The shipped lengths are
  not reliable: the `3DMD` length equals the record length in 14,918 records, the record length − 8 in 3,979, and
  something else in 24,018; chunk lengths usually equal the chunk's full size (tag and length included) but
  thousands do not (§3.14). A reader must not use them.
- A missing or misspelled tag fails the load. Chunks cannot be reordered, skipped or repeated, and unknown chunks
  are not allowed between them.
- After the last expected chunk the reader stops; anything after it is ignored. (No shipped record has trailing
  bytes.)
- Any getter failure (end of data) fails the load.

### 3.3 HEAD and versions *[compat]*

`u16` major, `u16` minor (after the tag and length). Accepted: major 0 with any minor, or major 1 with minor 0–5.
Anything else fails. Versions change the grammar in four places:

| Version | VERT block header | MATS mag/min filter bytes | PROP | REGP |
|---|---|---|---|---|
| 0.x | new (u32 format) | no | no | no |
| 1.0 | legacy (u16 format, u16 stride) | no | no | no |
| 1.1 | legacy | no | yes | no |
| 1.2, 1.3 | legacy | no | yes | yes |
| 1.4 | new | no | yes | yes |
| 1.5 | new | yes | yes | yes |

That 0.x takes the new VERT layout is how the test is written ("major 1 and minor ≤ 3" means legacy) *[quirk]*.
Shipped: 1.5 in 42,713 records, 1.3 in 202.

### 3.4 VERT *[compat]*

`u32` block count, then per vertex block:

```
u16   flags          read, ignored (always 0 in the data)
u16   vertex count   N
legacy (1.0–1.3):    u16 format, u16 stride S (stored as read)
new (0.x, 1.4, 1.5): u32 format; S = standard stride of the format (§3.5)
S × N bytes          interleaved vertices
```

- The block keeps (format, S, N, data). Its bounding box is **computed** from the first three floats of each vertex
  (step S; `0x00801040`); there is no bounding box in the file.
- At draw time the vertex pointer is set with stride 0, so the renderer uses the **standard** stride of the format,
  not S (`RenderMesh` `0x007D4A80`, `InterleavedArrays` `0x00881EC0`). A legacy block whose S differs from the
  standard stride is read with S and drawn with the standard one *[quirk]*. Every shipped legacy block has format 2
  and S = 20, which agree.
- A new vertex block starts as format 2, stride 20, with an empty box (min (1, 1, 1), max (0, 0, 0); `0x00802B90`).
  The bounding-box pass seeds the box from vertex 0 **without checking N or the buffer**, then takes N − 1 more.
  So N = 0, or a stride of 0, reads through an empty buffer (and N = 0 then loops about 2³² times) *[quirk]*; no
  shipped block has either.
- An unknown non-packed format gets stride 0 (§3.5), so no vertex bytes are read and the case above follows.
  FreeCity4 rejects an unknown format *[choice]*, and accepts N = 0 with an empty box *[choice]*.

### 3.5 Vertex formats *[compat]*

`StandardVFStride` (`0x008D0370`) and `PackStandardVertexFormat` (`0x008D01F0`) define the formats. A format with
bit 31 set is **packed**: bits 0–30 hold counts of ten element kinds. A format below `0x80000000` is a **standard**
number, translated to its packed form.

Packed layout (bit position, width, element size; `StandardVFElementOffset` `0x008D04A0` and its tables at
`0x00ACC0AC`):

| Kind | Bits | Size each | Meaning |
|---|---|---|---|
| 0 | 0–1 | 12 | position, 3 × f32 |
| 1 | 2–4 | 4 | 1 × f32 *[inferred: weight]* |
| 2 | 5 | 4 | 4 bytes *[inferred]* |
| 3 | 6 | 12 | normal, 3 × f32 |
| 4 | 7 | 4 | 4 bytes *[inferred]* |
| 5 | 8–9 | 4 | colour, 32-bit (byte order GF6) |
| 6 | 10–13 | 4 | 1-component texture coordinates |
| 7 | 14–17 | 8 | 2-component texture coordinates (u, v as f32) |
| 8 | 18–21 | 12 | 3-component texture coordinates |
| 9 | 22–25 | 16 | 4-component texture coordinates |

Elements are laid out in kind order (position first, then normal, colour, texture coordinates). The stride is the
sum of count × size (`0x008D03CD`), with one detail: kinds 1–4, 8 and 9 are counted only when bits 0–3 of the
format are not all zero, which holds whenever a position is present *[quirk]*.

`StandardVFElementOffset` places an element after the kinds before it, but its mask table is shifted by one: it
leaves out the kind immediately below (kind 1 even gets offset 0, on top of the position) *[quirk]*. This changes
nothing for the standard formats, where the kind just below each present element is always empty; FreeCity4 uses
the plain kind-order layout *[choice]*.

Standard formats (`PackStandardVertexFormat` table at `0x008D028C`):

| Standard | Packed | Elements | Stride |
|---|---|---|---|
| 1 | `0x80000101` | position, colour | 16 |
| 2 | `0x80004001` | position, uv | 20 |
| 3 | `0x80008001` | position, 2 uv | 28 |
| 10 | `0x80004101` | position, colour, uv | 24 |
| 11 | `0x80008101` | position, colour, 2 uv | 32 |
| 32 | `0x80000001` | position | 12 |
| 33 | `0x80000041` | position, normal | 24 |
| 34 | `0x80000141` | position, normal, colour | 28 |
| 35 | `0x80004041` | position, normal, uv | 32 |
| 36 | `0x80008041` | position, normal, 2 uv | 40 |
| 37 | `0x80004141` | position, normal, colour, uv | 36 |
| 38 | `0x80008141` | position, normal, colour, 2 uv | 44 |

Any other standard number packs to 0 (stride 0, §3.4). Shipped: `0x80004001` 165,256 blocks; standard 2 (new
layout) 1,526; standard 2 (legacy, S = 20) 286; `0x80004101` 2 blocks (one model). Vertex counts 1–1,898.

### 3.6 INDX *[compat]*

`u32` block count, then per index block:

```
u16   flags        read, ignored (always 0)
u16   index size   must be 2
u16   count        K
2 × K bytes        u16 indices (only when size = 2 and K > 0)
```

- Size 2, K = 0: the block is empty (no index buffer). 15,311 shipped blocks are.
- **Size ≠ 2: no index bytes are read**, and the reader goes on to the next block from the current position, so
  the rest of the model is misread and the load normally fails at the next tag *[quirk]*. FreeCity4 rejects the
  record *[choice]*. Every shipped block has size 2.

### 3.7 PRIM *[compat]*

`u32` block count, then per primitive block: `u16` count P and P × 12 bytes, each `u32` type, `u32` first,
`u32` count. With an index block, the mesh draws `count` indices starting at index `first` (`DrawElements(type,
count, u16 indices, indices + first)`); without one it draws `count` vertices from vertex `first`
(`DrawArrays`) (`RenderMesh` `0x007D4A80`). Types are the renderer's primitive enum (OpenGL table `0x00B12050`,
DX7 table `0x00AC44CC`):

| Type | Primitive |
|---|---|
| 0 | triangle list |
| 1 | triangle strip |
| 2 | triangle fan |
| 3 | points |
| 4 | lines |
| 5 | line strip |
| 6 | quads (DX7 converts) |
| 7 | quad strip (DX7 converts) |

The DX7 driver ignores types above 7. Shipped: 138,735 primitives, all type 0; every block has 0 or 1 primitive;
every primitive's index range fits its index block and every index fits its vertex block (checked for each frame).

### 3.8 MATS *[compat]*

`u32` material count, then per material:

```
u32   flags              only the low 16 bits are kept, then OR 0x180 (CreateMaterial 0x00800D70)
u8    alpha function     compare enum (below)
u8    depth function     compare enum
u8    source blend       blend enum
u8    destination blend  blend enum
u16   alpha threshold    only the high byte is kept
u32   material class     read, ignored
u8    (reserved)         read, then overwritten by the next byte
u8    texture count T
T ×   texture:
      u32   texture ID   instance of the FSH (§4.1); 0 = none
      u8    wrap S       texture-parameter enum
      u8    wrap T
      u8    mag filter   1.5 only; earlier versions: 0
      u8    min filter   1.5 only; earlier versions: 0
      u16   anim rate    read, ignored
      u16   anim mode    read, ignored
      u8    name length L
      L bytes name       read, ignored (shipped names include their NUL)
```

- **T > 2 overflows** the reader's two texture slots *[quirk]* (shipped T is 0 or 1). FreeCity4 rejects T > 2
  *[choice]*.
- If the **last** texture's ID is 0, T is set to 0 for the whole material *[quirk]* (2 shipped materials).
- Material flags (`SetRenderState` `0x007D5310`, applied by `0x007D4530`):

  | Bit | Meaning |
  |---|---|
  | `0x0001` | alpha test |
  | `0x0002` | depth test |
  | `0x0008` | back-face culling |
  | `0x0010` | blending |
  | `0x0020` | bind the material's textures (texturing itself follows T) |
  | `0x0080` | depth write (always set by the OR) |
  | `0x0100` | colour write (always set by the OR) |

  Other bits of the low 16 are not used *[inferred]*; bits 16–31 are dropped (20 shipped materials have
  `0x00010022`).
- Alpha reference = round(high byte / 255 × 15) / 15: the packed render state keeps 4 bits (`0x007D2B50`, constants
  1/255 at `0x00ABB4E0` and 15 at `0x00AB3CC8`). The shipped `0x7FFF` gives 7/15.
- Compare enum (OpenGL table `0x00B12070`): 0 never, 1 less, 2 equal, 3 less-or-equal, 4 greater, 5 not-equal,
  6 greater-or-equal, 7 always.
- Blend enum (`0x00B12090`): 0 zero, 1 one, 2 src colour, 3 1 − src colour, 4 src alpha, 5 1 − src alpha, 6 dst
  alpha, 7 1 − dst alpha, 8 dst colour, 9 1 − dst colour, 10 src alpha saturate.
- Texture-parameter enum (`0x00B12168`), used for wrap and filter bytes: 0 nearest, 1 linear, 2 clamp, 3 repeat,
  4 nearest-mipmap-nearest, 5 linear-mipmap-nearest, 6 nearest-mipmap-linear, 7 linear-mipmap-linear. Wrap uses 2
  and 3. The draw context may override both filters (`SetTexFiltering` `0x007D4280`) and the wrap modes
  (`0x007D41E0`) from three boolean render properties (GF5):
  - `kForcePointSampling` (boolean property 5): (nearest, nearest);
  - otherwise `kForceBilinear` (boolean property 6): (linear, linear-mipmap-linear), despite its name;
  - `kForceTextureRepeat` (boolean property 21): both wraps repeat.

  For a stage above 0, neither function does anything unless `kUseSecondStage` (boolean property 11) is on. The
  three force flags are registered false (`0x007DF320`, `0x007DF400`, `0x007E0120`) and the shipped
  `Graphics Rules.sgr` never sets them, so **in normal play there is no override** and the file's bytes are used.
  The 137,480 shipped 1.5 textures with filters (0, 0) are therefore drawn with nearest sampling and no
  mipmapping (DX7: magnification and minification point, mip filter none, `0x00884363`–`0x008843BB`).
- The context's highlight modes can override blending, depth and alpha function; those are draw states, not data.

### 3.9 ANIM *[compat]*

```
u16   frame count F        model +0x48
u16   frame rate           stored
u16   anim mode            stored as a byte
u32   flags                stored as a byte
f32   displacement         stored
u16   mesh count           then the mesh list is sized to it
per mesh:
      u8    name length L
      u8    mesh flags
      L bytes name          made a string with C-string semantics: the stored length must include the NUL
                            (every shipped name does)
      F × 8 bytes           per frame four s16: VERT block, INDX block, PRIM block, MATS material
```

- **Validation** (`0x00807C00`): each of the four indices of every frame must be ≥ 0 and below its chunk's count.
  **One bad index drops the whole mesh**, silently; the load continues and the mesh count becomes the number of
  meshes kept *[compat]*. Shipped: 71 meshes are dropped. All of them have (−1, −1, −1, m) in every frame, so they
  would never show anyway.
- A model with F frames can be drawn at frame 0 … F − 1; asking for frame ≥ F fails (`GetModelFrame`).
- Frame rate, mode, flags and displacement are stored for the animated model instance (not traced here, GF10).
  Shipped: F = 1 in 41,975 models, F > 1 in 940; rate 30 (30,157), 0 (12,491), 10, 3, 2, 1; mode always 3; flags
  and displacement always 0; mesh flags always 0.

### 3.10 PROP (1.1 and later) *[compat]*

`u32` count, then per entry: `s16` mesh index, `s16` frame, `u8` length + key string, `u8` length + value string
(both as in §3.9, NUL included). The array is kept on the model and sorted after reading (`0x00806B40`).
Shipped: 58 models with 2 entries each, e.g. (−1, 0, `skeleton`, `adult`) and (−1, 0, `suit`,
`RangerFFit_01`). Consumer not traced (GF11).

### 3.11 REGP (1.2 and later) *[compat]*

`u32` count, then per entry: `u8` length + name (NUL included; **lower-cased** on read), `u16` count C, C × 28
bytes (7 × f32 each). Entries go into a map keyed by the name; a repeated name appends to the existing list.
Shipped: 1,810 models have entries, names such as `fx_00_highway_bridge_collumn_f` and
`fx_00_blue_blinking_light_slow`, C = 1. The 7 floats are a position and an orientation *[inferred]*; consumer not
traced (GF11).

### 3.12 Building and drawing a frame *[compat]*

`GetModelFrame` (`0x00806C40`), for frame f < F, makes one mesh per animated mesh: vertex block, index block and
primitive block from the frame's indices, and the material (binding its textures on first use, §4.1). A negative
material index would hide the mesh, but the load validation (§3.9) never lets one through. `RenderMesh`
(`0x007D4A80`) skips a mesh with no vertex block or zero vertices, sets the vertex pointer (format, standard stride)
and draws each primitive (§3.7) with 16-bit indices.

### 3.13 What the reader ignores

All chunk lengths; the VERT and INDX block flags; the material class, reserved byte, texture anim rate and mode and
texture names; the high 16 bits of material flags; the low byte of the alpha threshold; anything after the last
expected chunk. The bounding box is computed, never read.

### 3.14 S3D survey

| Measure | Result |
|---|---|
| Records | 42,915, group `0xBADB57F1`; versions 1.5: 42,713, 1.3: 202; 0 failures; no trailing bytes |
| `3DMD` length | = record length 14,918; = record length − 8: 3,979; other: 24,018 |
| `HEAD` length | always 12 (the chunk's full size) |
| Other chunk lengths | equal to the full chunk size in most records (e.g. VERT 32,325, PROP 42,857, REGP 40,595); the rest disagree in both directions |
| Blocks per model | one block per chunk in 30,451–30,658 models; up to 1,440 vertex blocks (animated models) and 67 index blocks, primitive blocks and materials |
| VERT | 167,070 blocks; formats §3.5; counts ≤ 16: 124,706, ≤ 256: 40,656, ≤ 1,898: 1,708 |
| INDX | 155,661 blocks, size 2, flags 0; 15,311 empty |
| PRIM | 154,046 blocks; 138,735 primitives, all type 0 (triangle list) |
| MATS | 155,827 materials. Flags `0xAB` 119,220; `0xA3` 20,245; `0xAA` 14,001; `0xA2` 1,320; `0xBB` 528; `0xB3` 225; `0x3B` 149; `0x33` 116; `0x10022` 20; `0x8A` 3. Alpha function 4 and depth function 3 everywhere. Blend (1, 0) 154,789; (4, 5) 730; (8, 1) 308. Threshold `0x7FFF` 154,484, 0 1,343. Class 0, reserved 0. T = 1 (155,824) or 0 (3) |
| Textures | wrap (3, 3) 155,688; (2, 2) 96; (2, 3) 40. Filters (1.5): (0, 0) 137,480; (1, 1) 18,118. Anim (33, 2) 155,637; (100, 2) 129; (0, 2) 58. All names NUL-terminated |
| ANIM | 155,827 meshes; 71 dropped by validation (§3.9) |
| PROP / REGP | 58 / 1,810 models with entries |

---

## 4. Models, textures and exemplars

### 4.1 Material texture → FSH key *[compat]*

The first time a frame uses a material, each texture slot with a non-zero ID and no binding is resolved
(`GetModelFrame` `0x00806C40`) through the texture binding factory:

1. (`0x7AB50E44`, `0x1ABE787D`, ID): slot `0x14` (`0x008094C0`) builds this key with the group hard-coded.
2. If that fails: (`0x7AB50E44`, the model's own group, ID) through slot `0x18` (`0x0080A380`).
3. If that fails too: instance `0x07F94200` in group `0x1ABE787D` (shipped in `SimCity_3.dat`, 32 × 32 DXT1), and
   the slot's ID is set to 0 so it is not looked up again.

The binding cache keys on (group, instance) only (§2.10). On the shipped data, step 1 always succeeds.

### 4.2 Exemplar model keys *[compat]*

An occupant's exemplar names its model with one of the resource-key-type properties
(`cSC4ModelMakerUtility::GetModelInstanceID` `0x00496830`). Let the property's values be v[0], v[1], …, and z the
zoom after the detail remap: z = table[zoom + 5 × detail] (table `0x00B094A4`: detail 0 → 0, 1, 2, 3, 4; detail 1 →
0, 1, 2, 3, 3; detail 2 → 0, 0, 1, 2, 3; detail global `0x00B216FC`, below). Let r be the rotation, 0–3, relative
to the view *[inferred naming]*: the function forms (3a + b) mod 4 from its two rotation arguments a and b, or
3a mod 4 when its flag argument is set.

The detail is the integer render property `kModelLODOffset` (integer property 22), clamped to 0–2 (GF8). The model
maker reads it in `Init` (`0x004948BE`–`0x004948DF`) and again on message `0xC9F775BB` (`0x00496466`), which it
subscribes to in `DoPostCityInit` (`0x004952B8`) and which `cSC4App::ApplyVideoPreferences` (`0x0044A2F3`) and
`cSC4View3DWin::SetRenderProperties` (`0x007ECF15`) post. `Graphics Rules.sgr` sets `modelLODOffset` from the
Texture Quality option: High 0, Medium 1, Low 2; the registered default is 0 (`0x007E2330`). Example: at Low, zoom 4
gives z = table[4 + 5 × 2] = 3, so an RKT1 model uses instance v[2] + `0x300` + r × `0x10`, the zoom-3 model.

| Property | Key |
|---|---|
| `0x27812820` (RKT0) | (v[0], v[1], v[2]) for every zoom and rotation |
| `0x27812821` (RKT1) | (v[0], v[1], v[2] + z × `0x100` + r × `0x10`); v[2] = 0 means no model |
| `0x27812822` (RKT2) | (v[0], v[1], v[2 + 4z + r]): 20 explicit instances |
| `0x27812823` (RKT3) | (v[0], v[1], v[2 + z]): one per zoom |
| `0x27812824`, `0x27812825` (RKT4, RKT5) | not handled by this function (other callers, GF7) |

Shipped S3D instances follow the RKT1 pattern: of 2,761 instance bases (instance with the low 12 bits cleared),
1,957 hold exactly the 20 variants zoom 0–4 (bits 8–11) × rotation 0–3 (bits 4–7); the low 4 bits are 0 in 42,388
of 42,915 models.

---

## 5. Where the exe does it

| Address | Function |
|---|---|
| `0x00457EC5`–`0x00457FE8` | resource factory registrations (FSH `0x00457F16`/`0x00457F2A`, S3D `0x00457F66`) |
| `0x00800B80` | FSH factory `CreateInstance` (type `0x7AB50E44` only) |
| `0x00800860` | FSH factory `SerialRead`: type check, accept rules, level tables (§2.7, §2.8) |
| `0x00800670` | code → (format, type) (§2.6) |
| `0x00808F00` / `0x00808FD0` | `cS3DShapeFile::Open` / `OpenInfoOnly` (§2.2, §2.9) |
| `0x00808CE0` | select frame, palette search (§2.5) |
| `0x00808D70` / `0x00808DE0` | entry header / palette info (§2.4, §2.5) |
| `0x00808E40` | `PixelWidthMul2` |
| `0x00809B90` | upload levels per frame (§2.10) |
| `0x008094C0`, `0x0080A380` | binding by ID (shared group) / by key (§4.1) |
| `0x0088D0B0` | DX7 format translation (§2.6) |
| `0x008065C0` | S3D factory `CreateInstance` |
| `0x00806FA0`–`0x008085AF` | S3D `SerialRead` (HEAD `0x00807066`, VERT `0x008070FF`, INDX `0x008072EB`, PRIM `0x0080749E`, MATS `0x0080760A`, ANIM `0x0080798D`, PROP `0x00807EED`, REGP `0x00808022`) |
| `0x00801040` | vertex block bounding box |
| `0x008D0370`, `0x008D01F0`, `0x008D04A0` | vertex format stride, packing, element offset (§3.5) |
| `0x00800D70` | `CreateMaterial` (flags, alpha reference byte, texture slots) |
| `0x00806C40` | `GetModelFrame` (§3.12, §4.1) |
| `0x007D4A80` | `RenderMesh` (§3.7) |
| `0x007D5310`, `0x007D2B50`, `0x007D4530` | material → packed render state → driver (§3.8) |
| `0x00496830` | model key from an exemplar property (§4.2) |

---

## 6. Interfaces and replaceability

| Interface | Class | Vtable | Notes |
|---|---|---|---|
| resource factory | FSH (class `0x3AB50E2A`) | `0x00ABD328` | `CreateInstance` slot 3 `0x00800B80`, `SerialRead` slot 9 `0x00800860` |
| resource factory | S3D (class `0xFAD0F0B6`) | entries at `0x00ABD5B8` / `0x00ABD5BC` | create `0x008065C0` (type check, new `cS3DResModel`), `SerialRead` `0x00806FA0`; slot numbers not established |
| texture binding factory | class `0x1AC0E11A`, IID `0xFAC0E219` | `0x00ABD8C8` | slot `0x14` `0x008094C0` (by ID), slot `0x18` `0x0080A380` (by key) |
| material manager | service `0x1AC0E11B`, IID `0xFAC0E21A` | `0x00ABD3F4` | slot `0x0C` `CreateMaterial` `0x00800D70` |

Replaceability: both resource factories are registered by type, so a replacement factory registered for
`0x7AB50E44` or `0x5AD0E817` takes over loading. Its objects are consumed through the original resource
interfaces by the binding factory and by `GetModelFrame`, which read the model's memory layout directly
*[inferred]*, so in plug-in mode FreeCity4 keeps the original classes. In standalone mode FreeCity4 replaces the
readers wholesale (§10) and the renderer consumes the decoded data.

---

## 7. Quirks and choices (summary)

| # | Behaviour | Tag |
|---|---|---|
| Q1 | FSH type `0x7AB50E45` registered but never created | *[compat]* |
| Q2 | FSH `size` and offsets not bounds-checked; a short record fails, extra bytes ignored | *[quirk]*, FreeCity4 checks *[choice]* |
| Q3 | Height must be a power of two (0 passes); width unchecked; frames compared on height, code, mips only | *[compat]* |
| Q4 | Levels 16-byte aligned; DXT levels stop at 4 × 4; unpadded shipped DXT1 mips misread | *[compat]* *[quirk]* |
| Q5 | 12-bit field A selects DXT1 RGB/RGBA; DX7 treats both alike | *[compat]*, decode as DX7 *[choice]* |
| Q6 | Codes `0x64 0x65 0x6D 0x78 0x7B 0x7E` load but cannot be created under DX7 (drawn untextured); signed BYTE under GL | *[compat]* *[quirk]*, FreeCity4 decodes them *[choice]* |
| Q7 | `0x7B` needs a 256 × 1 palette but uploads the indices; no palette → uninitialised check | *[quirk]*, apply palette / reject *[choice]* |
| Q8 | Only DBPF-level compression; bit-7 codes rejected | *[compat]* |
| Q9 | S3D chunk and container lengths ignored; fixed chunk order | *[compat]* |
| Q10 | Version 0.x uses the new VERT layout; legacy stride stored but the draw uses the standard stride | *[quirk]* |
| Q11 | Unknown vertex format → stride 0 → bounding box reads an empty buffer | *[quirk]*, reject *[choice]* |
| Q12 | INDX size ≠ 2 reads nothing and misaligns the stream | *[quirk]*, reject *[choice]* |
| Q13 | MATS: 16-bit flags OR `0x180`; alpha reference 4 bits; T > 2 overflows; last ID 0 clears T | *[compat]* *[quirk]*, reject T > 2 *[choice]* |
| Q14 | Names are C strings: the stored length must include the NUL | *[compat]* |
| Q15 | A mesh with any invalid frame index is dropped silently | *[compat]* |
| Q16 | Texture lookup: shared group, then the model's group, then `0x07F94200` | *[compat]* |
| Q17 | Decoding rules of §2.11 (CPU decode; DXT rounding; 16-bit layouts) | *[choice]* |

---

## 8. Conformance tests

Byte strings are hexadecimal. "FSH(entry)" is a one-entry record: `53 48 50 49`, `size` (the record length),
`01 00 00 00`, `47 32 36 34` (`G264`), the directory entry `30 30 30 30 18 00 00 00`, then the entry at `0x18`
(16-byte header, then pixels). Decoded pixels are (R, G, B, A), row-major. "Accepted" means the factory's
`SerialRead` succeeds; "format, type" is §2.6's pair.

### 8.1 FSH

1. **DXT1, 4-colour.** FSH(`60 00 00 00 04 00 04 00 00 00 00 00 00 00 00 00` + `00 F8 1F 00 E4 E4 E4 E4`),
   size `0x30`. Accepted; format 7, type 0; one level at pixel offset 0, 8 bytes. Every row decodes to
   (255, 0, 0, 255), (0, 0, 255, 255), (170, 0, 85, 255), (85, 0, 170, 255).
2. **DXT1, 3-colour, A = 1.** Header as test 1 but `+0x0C` = `01 00 00 00`; pixels `1F 00 00 F8 F4 F4 F4 F4`
   (c0 = blue ≤ c1 = red; indices 0, 1, 3, 3). Format 8. Every row: (0, 0, 255, 255), (255, 0, 0, 255),
   (0, 0, 0, 0), (0, 0, 0, 0). With A = 0 the format is 7 and FreeCity4 decodes the same pixels (§2.11).
   A = `FFF` (−1 as 12 bits) also gives format 8.
3. **DXT3.** FSH(`61 00 00 00 04 00 04 00 00 00 00 00 00 00 00 00` + `10 32 54 76 98 BA DC FE 00 F8 00 F8 00 00 00 00`),
   size `0x38`. Format 9. Pixel i (0–15, row-major) = (255, 0, 0, 17 i).
4. **DXT5.** Code `0x62`, 4 × 4, pixels `FF 00 08 82 20 08 82 20 00 F8 00 F8 00 00 00 00`, size `0x38`. Format 10.
   Pixel i = (255, 0, 0, 255) for even i, (255, 0, 0, 0) for odd i.
5. **8888.** FSH(`7D 00 00 00 02 00 02 00 00 00 00 00 00 00 00 00` + `FF 00 00 80 00 FF 00 FF 00 00 FF 00 11 22 33 44`),
   size `0x38`. Format 3, type 1; level 0 is 16 bytes. Pixels (0, 0, 255, 128), (0, 255, 0, 255),
   (255, 0, 0, 0), (`33`, `22`, `11`, `44`).
6. **888.** Code `0x7F`, 2 × 2, pixels `FF 00 00 00 FF 00 00 00 FF 10 20 30`, size `0x34`. Format 2, type 1;
   level 12 bytes. Pixels (0, 0, 255, 255), (0, 255, 0, 255), (255, 0, 0, 255), (`30`, `20`, `10`, 255).
7. **8-bit with palette.** Code `0x7B`, 2 × 2, `next` = `0x20` (header bytes `7B 20 00 00 02 00 02 00 00 00 00 00
   00 00 00 00`), pixels `00 01 80 FF`, 12 zero bytes, then the palette block `24 00 00 00 00 01 01 00 00 01 00 00
   00 00 00 00` (at `0x38`) and 768 bytes where entry k is (k, k, k); size `0x348`. Accepted, format 5, type 0.
   FreeCity4 pixels: (0, 0, 0, 255), (1, 1, 1, 255), (128, 128, 128, 255), (255, 255, 255, 255).
   - 7a. The same with palette width 255 (`FF 00` at palette +4): rejected.
   - 7b. The same with the palette code `0x22`: no palette is found (`0x22` is not searched): FreeCity4 rejects.
   - 7c. Palette code `0x2A` with 1,024 bytes (entry k = `kk kk kk FF`), size `0x448`: accepted; same pixels.
   - 7d. Test 7 with `next` = 0 (palette unreachable): FreeCity4 rejects (the original reads uninitialised data).
8. **4444.** Code `0x6D`, 2 × 2, pixels `00 8F F0 00 0F 00 FF FF` (`u16` `0x8F00`, `0x00F0`, `0x000F`, `0xFFFF`).
   Accepted, format 3, type 8. FreeCity4: (255, 0, 0, 136), (0, 255, 0, 0), (0, 0, 255, 0), (255, 255, 255, 255).
9. **1555.** Code `0x7E`, 2 × 2, `u16` `0xFC00`, `0x03E0`, `0x801F`, `0x7FFF`. Format 3, type 9. FreeCity4:
   (255, 0, 0, 255), (0, 255, 0, 0), (0, 0, 255, 255), (255, 255, 255, 0).
10. **565.** Code `0x78`, 2 × 2, `u16` `0xF800`, `0x07E0`, `0x001F`, `0x0000`. Format 2, type 11. FreeCity4:
    (255, 0, 0, 255), (0, 255, 0, 255), (0, 0, 255, 255), (0, 0, 0, 255).
11. **8-bit alpha / luminance-alpha.** Code `0x64`, 2 × 2, pixels `00 55 AA FF`: format 4; FreeCity4
    (255, 255, 255, a) for a = 0, 85, 170, 255. Code `0x65`, 2 × 2, pixels `10 20 30 40 50 60 70 80`: format 6;
    FreeCity4 (`10`,`10`,`10`,`20`), (`30`,`30`,`30`,`40`), (`50`,`50`,`50`,`60`), (`70`,`70`,`70`,`80`).
12. **Rejected codes.** Test 1 with code `0x7A`, `0x63`, `0x6E`, `0x80`, `0xE0`: rejected.
13. **Height and width.** Test 5 with height 3: rejected. Test 5 with height 0: accepted (0 passes the
    power-of-two test); the texture is 2 × 1 and level 0 is the first 8 pixel bytes. Code `0x7D`, width 3,
    height 2, 24 pixel bytes: accepted (width unchecked), level 0 = 8 × 3 × 2 / 2 = 24 bytes.
14. **Level layout.** DXT1 8 × 8 with M = 3: level offsets 0, 32, 48, 64, sizes 32, 8, 8, 8. 8888 4 × 4 with
    M = 2: offsets 0, 64, 80, sizes 64, 16, 4. DXT3 2 × 2 (clamped to 4 × 4), M = 1: offsets 0, 16, sizes 16, 16.
15. **Frames.** Two entries (directory offsets `0x20` and `0x40` in a record laid out accordingly), both code
    `0x7D` 2 × 2: accepted, 2 frames. Second entry with height 4: rejected. Second entry with code `0x7F`:
    rejected. Second entry with M = 1: rejected. Second entry with width 4: accepted; its level 0 is read as
    2 × 2 (16 bytes).
16. **Size field.** Test 5 with the record extended by 4 bytes (size unchanged): accepted, the extra bytes are
    ignored. Test 5 with size `0x3C` (record still `0x38`): rejected (the read fails).
17. **Magic.** Test 5 with `53 48 50 48`: rejected. A DIR-listed QFS-compressed copy of test 5: accepted after
    data-formats §2.6–§2.7 decompression; the same bytes **not** DIR-listed: rejected (the first four bytes are the
    compressed-length prefix of data-formats §2.7, not `SHPI`).
18. **Type.** The bytes of test 5 under type `0x7AB50E45`: no resource is created.
19. **Unpadded mips (shipped).** `SimCity_2.dat` (`0x7AB50E44`, `0x1ABE787D`, `0x10EC0000`): DXT1 128 × 128, M = 7,
    A = 1: the original's level offsets are 0, 8192, 10240, 10752, 10880, 10912, 10928, 10944 from the pixel
    start; the pixel data ends at 10944 (the next block's header starts there), so the last level's 8 bytes are
    `70 00 00 00 31 30 45 43`, the start of the `0x70` name block.
20. **Fallback texture (shipped).** `SimCity_3.dat` (`0x7AB50E44`, `0x1ABE787D`, `0x07F94200`): 576 bytes, header as
    §2.2's example, one entry: code `0x60`, 32 × 32, M = 0; accepted, format 7.

### 8.2 S3D

The test model T is version 1.5, one triangle. Fields in order (hex):

```
"3DMD" FC 00 00 00                                       length 252, the record (ignored)
"HEAD" 0C 00 00 00  01 00  05 00                         1.5
"VERT" 50 00 00 00  01 00 00 00                           1 block
       00 00  03 00  01 40 00 80                          flags, 3 vertices, format 0x80004001
       (0,0,0, 0,0)  (1,0,0, 1,0)  (0,1,0, 0,1)            15 f32: 00 00 00 00 / 00 00 80 3F
"INDX" 18 00 00 00  01 00 00 00  00 00  02 00  03 00      1 block, size 2, 3 indices
       00 00 01 00 02 00
"PRIM" 1A 00 00 00  01 00 00 00  01 00                    1 block, 1 primitive
       00 00 00 00  00 00 00 00  03 00 00 00              type 0, first 0, count 3
"MATS" 2A 00 00 00  01 00 00 00                           1 material
       AB 00 00 00  04 03 01 00  FF 7F  00 00 00 00  00  01
       78 56 34 12  03 03 01 01  21 00 02 00  01 00       ID 0x12345678, repeat, linear, name "\0"
"ANIM" 24 00 00 00  01 00 1E 00 03 00  00 00 00 00  00 00 00 00  01 00
       02 00 6D 00  00 00 00 00 00 00 00 00               mesh "m\0", frame (0, 0, 0, 0)
"PROP" 0C 00 00 00  00 00 00 00
"REGP" 0C 00 00 00  00 00 00 00
```

(The ANIM mesh line is: name length 2, flags 0, `6D 00`, then the frame.)

21. **T loads.** 1 frame, 1 mesh. Vertex block: format `0x80004001`, stride 20, 3 vertices, bounding box (0, 0, 0)
    to (1, 1, 0). Frame 0 draws `DrawElements`(triangle list, 3, u16, [0, 1, 2]). Material: flags `0x1AB`, alpha
    greater with reference 7/15, depth less-or-equal, blend (one, zero), one texture (`0x7AB50E44`, `0x1ABE787D`,
    `0x12345678`), wrap repeat/repeat, filters linear/linear. Asking for frame 1 fails.
22. **Lengths ignored.** T with every length field (including `3DMD`'s) set to 0 or `FFFFFFFF`: loads identically.
23. **Version 1.3.** T with minor 3, the VERT block header `00 00 03 00 02 00 14 00` (flags, count, standard
    format 2, stride 20) and the texture without the two filter bytes: loads; filters 0/0 (nearest), same
    geometry. With stride 24 in the block header: reads 72 vertex bytes (so T must carry 72), and draws with stride
    20 *[quirk]*.
24. **Version 1.0.** T with minor 0, legacy VERT and no filter bytes, and **without** PROP and REGP: loads. With
    PROP and REGP left in: also loads (the bytes after ANIM are ignored).
25. **Version 1.1.** T with minor 1, legacy VERT, no filter bytes, PROP kept and REGP removed: loads; with PROP
    removed: fails.
26. **Version 0.x.** T with major 0, minor 9, new VERT layout, no filter bytes, PROP and REGP removed: loads.
27. **Rejected versions.** Minor 6; major 2: fail.
28. **Chunk order.** T with INDX before VERT, or with a 4-byte `"XXXX"` chunk inserted before ANIM: fail.
29. **INDX size.** T with index size 4 (`04 00`): the index bytes are not consumed, the next tag read is
    `00 00 01 00` and the load fails. FreeCity4: rejects at the INDX block.
30. **Empty index block.** T with INDX count 0 and no index bytes, PRIM unchanged: loads; frame 0 draws
    `DrawArrays`(triangle list, first 0, count 3).
31. **Dropped mesh.** T with the frame's VERT index `FF FF`: loads; mesh count 0. With PRIM index 1 (only one
    PRIM block): same.
32. **Texture count.** T with T = 0 (the texture removed): loads, material untextured. T with texture ID 0: loads,
    T becomes 0. T with T = 3: FreeCity4 rejects (the original overflows).
33. **Flags.** T with flags `0x00010022`: material flags `0x01A2` (bits above 15 dropped, `0x180` added).
34. **Alpha reference.** Threshold `0x8000` → 8/15; `0x0000` → 0; `0xFF00` → 15/15; `0x087F` → 0 (high byte 8:
    8/255 × 15 = 0.47, rounds to 0).
35. **Vertex formats.** T with format 2 (new layout): stride 20, loads identically. With `0x80004101` and 24-byte
    vertices (a colour dword after the position): stride 24, uv at byte 16. With standard format 4: FreeCity4
    rejects (original stride 0, §3.4).
36. **Strings.** T with the mesh name length 1 and bytes `6D` (no NUL): FreeCity4 reads the name `m` *[choice]*;
    the original reads past the byte as a C string *[quirk]*.
37. **REGP.** T with REGP count 2: entries (`"FX_A\0"`, 1 × 28 bytes) and (`"fx_a\0"`, 1 × 28 bytes): one map
    entry `fx_a` with two transforms.

### 8.3 Links and keys

38. **Texture lookup.** A model in group G with texture ID X: if (`0x7AB50E44`, `0x1ABE787D`, X) exists it is
    used; else if (`0x7AB50E44`, G, X) exists it is used; else (`0x7AB50E44`, `0x1ABE787D`, `0x07F94200`) is
    bound and the slot's ID becomes 0.
39. **RKT1.** Values (`0x5AD0E817`, `0xBADB57F1`, `0x12340000`), zoom 5 (index 4), detail 0, rotation 2:
    instance `0x12340420`. Detail 1: `0x12340320`. Detail 2: `0x12340320`. Instance value 0: no model.
40. **RKT2.** Values (T, G, I0 … I19), zoom index 1, rotation 3, detail 0: instance I7.
41. **RKT3.** Values (T, G, I0 … I4), zoom index 2, detail 2: instance I1.

### 8.4 Shipped-data sweep

42. Every FSH record of the install (§2.13 counts) is accepted; per code the entry counts are `0x61` 25,957,
    `0x60` 19,357, `0x7D` 40, `0x7B` 24, `0x7F` 9; 167 DXT1 entries read their last level from the
    following name block (§2.8).
43. Every S3D record (42,915) loads; 167,070 vertex blocks, 155,661 index blocks, 138,735 primitives, 155,827
    materials; 71 meshes dropped; every texture ID resolves at step 1 of §4.1 or is 0.

---

## 9. Community descriptions versus the exe

Common modding descriptions of FSH and S3D were the starting point. Where they differ, the exe wins:

| Topic | Commonly described | The exe |
|---|---|---|
| FSH entry `+0x0C`/`+0x0E` | x and y "position" fields, mip count in the top 4 bits of `+0x0E` | mip count is bits 28–31 of the dword (same bits); the low 12 bits are the DXT1 RGB/RGBA selector (§2.4); nothing else is read |
| FSH codes | 0x6D 4444, 0x78 565, 0x7E 1555, 0x7B 8-bit, compressed-bitmap flag `0x80` | the factory accepts these (and `0x64`, `0x65`), but DX7 creates only DXT and 8888/888; bit `0x80` is rejected, not decompressed (§2.6, §2.12) |
| FSH palettes | several palette codes (incl. `0x22`) | only `0x2A`, `0x24`, `0x2D`, `0x29`, only after `0x7B`/`0x7A`, and the palette is never applied (§2.5, §2.6) |
| FSH directory ID, centre fields | meaningful | ignored |
| FSH mipmaps | levels follow each other | each level 16-byte aligned; DXT levels stop shrinking at 4 × 4 (§2.8) |
| S3D lengths | chunk sizes | ignored, and unreliable in the shipped data (§3.2) |
| S3D VERT | "1.3 and earlier: u16 format + u16 stride" | true for 1.0–1.3; 0.x uses the new layout; the drawn stride is always the format's standard one (§3.3–§3.5) |
| S3D INDX stride | any | must be 2, otherwise the stream misaligns (§3.6) |
| S3D materials | class, reserved, anim rate/mode, texture name are attributes | all read and ignored; flags keep 16 bits OR `0x180`; at most 2 textures (§3.8) |
| S3D frame index −1 | "hidden in this frame" | any negative index drops the whole mesh at load (§3.9) |
| Bounding box | sometimes described as stored | computed from positions (§3.4) |

---

## 10. engine/formats checklist (for the next lane)

`engine/formats` has no FSH or S3D reader today. Add:

1. **`fsh` module**: `SHPI` header, directory, entry header (§2.4), attachment chain with the palette search
   (§2.5), the code table (§2.6), the accept rules (§2.7), level offsets with 16-byte alignment read from the
   record (§2.8), frames. Expose per frame the (format, type) pair of §2.6, the level table, and a CPU decoder to
   RGBA8 (§2.11) for every accepted code. Bounds-check everything (Q2).
2. **`s3d` module**: the chunk grammar of §3.2–§3.11 with the version table, legacy VERT, the vertex-format
   tables (§3.5, packed and standard), INDX size rule, PRIM, MATS (16-bit flags OR `0x180`, alpha reference
   quantisation, T ≤ 2), ANIM with mesh dropping, PROP, REGP (lower-cased names, merging). Expose `frame(f)` →
   meshes (vertex block, index block, primitives, material) as §3.12, and the computed bounding boxes. Ignore all
   lengths. Reject Q11, Q12, Q13 cases *[choice]*.
3. **Key helpers**: texture lookup order of §4.1 (shared group, model group, `0x07F94200`) and the RKT0–RKT3
   instance arithmetic of §4.2 with the detail table.
4. **Tests**: §8.1–§8.3 as unit tests on synthetic fixtures built in code; §8.4 in `tests/game_data.rs` (sweep
   every FSH and S3D, compare the counts).
5. **`fc4-dat`**: `fsh <file> <T> <G> <I>` (header, entries, levels; optional PNG/TGA dump of level 0 for local
   inspection only) and `s3d <file> <T> <G> <I>` (chunks and counts).
6. **README**: record the decoding choices (§2.11, Q5–Q7, Q17) and the community differences of §9.

---

## 11. Open questions

Also filed in QUESTIONS.md, "From the graphics formats spec", GF1–GF12.

1. GF1: what a model shows when its texture cannot be created under DX7 (codes of Q6).
   **Answered** (§2.6): drawn untextured, in its diffuse colour.
2. GF2: channel layout of `0x6D`/`0x7E`/`0x78` (DX7 cannot create them; GL enums differ from the D3D layouts).
   **Answered** (§2.11): no renderer shows them correctly; follow the Direct3D layouts.
3. GF3: whether DX7 really shows DXT1 index-3 texels as transparent when A = 0.
   **Answered** (§2.11): yes; alpha-tested or blended materials drop them, others show them black.
4. GF4: who uses the 24 `0x7B` grey-ramp entries in group `0x891B0E1A`, and through which path.
5. GF5: defaults of the draw settings that override material filters and wrap.
   **Answered** (§3.8): `kForcePointSampling`, `kForceBilinear`, `kForceTextureRepeat`; all off.
6. GF6: byte order of the vertex colour element.
7. GF7: RKT4/RKT5 layouts.
8. GF8: where the detail global `0x00B216FC` is set.
   **Answered** (§4.2): `kModelLODOffset` clamped to 0–2, set by the Texture Quality option.
9. GF9: unpadded DXT1 mips: reproduce or fix.
   **Answered** (§2.8): reproduce, with the record's own bytes; zero only past `size`.
10. GF10: semantics of ANIM frame rate, mode, flags and displacement.
11. GF11: consumers of PROP and REGP, and the meaning of REGP's 7 floats.
12. GF12: whether anything reads the FSH centre fields or the bits 12–27 of `+0x0C`.

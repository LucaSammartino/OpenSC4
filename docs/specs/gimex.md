# Gimex image files: the factory, reader and writer (behaviour spec)

Scope: the owners `cGZGimexFactory` (48 functions, 6,002 bytes), `cGZGimexReader` (13, 980) and `cGZGimexWriter`
(10, 774) as tools/coverage.py assigns them: the framework's front end to the linked Gimex image library. The
factory loads image files, records and resources into `cIGZBuffer`s (converting the pixels to the buffer's colour
type) and saves buffers as image files. The Gimex library itself (`_GIMEX_*`, its codecs, file formats and
encoders) is a library contract, outside this spec *[choice: FreeCity4 uses its own decoders and encoders for the
formats it needs]*.

Related: span-render-4444.md §2 (colour types and where image loads pick them), persistence.md §6.1 (colour type →
bits), sl-library.md §3 (`cSLBuffer`, a factory user), persist-buffer-factory.md, render3d.md (`TakeSnapshot` saves
through the factory), framework-rz-files.md (`cRZFile`), intro-video.md and hit-music.md (the record-as-file object,
class `0x0A32C0DA`).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the three owners was read in the decompile; the vtables `0x00ACE530`,
`0x00ACE4FC`, `0x00ACE57C`, `0x00ACE5B0`, the pixel converters, the save converter `0x0090BA37` and the dither
`0x0090BF3C` were read in the disassembly. Written 2026-10-24 (re-gimex7).

Colour types (span-render-4444.md): 2 8-bit, 4 X1R5G5B5, 5 R5G6B5, 6 A1R5G5B5, 7 A4R4G4B4, 8 X8R8G8B8, 9 A8R8G8B8.
A 32-bit Gimex pixel is the little-endian dword A8R8G8B8 (bytes B, G, R, A). A Gimex frame description ("info")
holds the width `+0x10`, height `+0x14`, bits per pixel `+0x18` and, for 8-bit images, a 256-entry palette at
`+0x28` (4 bytes per entry: B, G, R, A).

## 1. Codec selection *[compat]*

Image codecs are numbered by the library. Wherever a codec number is taken, these values are special:

| Value | Meaning |
|---|---|
| `0x32` | detect by content (read only): ask every codec 1…49 how well it recognises the input (`_GIMEX_is`) and use the best positive score; none → fail |
| `0x33` | by the file's extension (the input's path, through the library's name lookup); no match → fail |
| `0x34` | by extension, else (read only) by content |
| other | that codec |

Writing accepts `0x33` and `0x34` (by extension); `0x32` alone fails. All library calls are made under one global
lock (`0x00B625A4`) after selecting the codec.

## 2. Reader and writer *[compat]*

Both hold a reference count (`+0x14`; `Release` at a count of 1 or less runs `FinalRelease`), the file (`+4`), an
"open" byte (`+8`) and the library context (`+0x0C`, `+0x10`).

| Reader slot (`0x00ACE57C`) | Writer slot (`0x00ACE5B0`) | Behaviour |
|---|---|---|
| 0 `QueryInterface` (`0x0090C62F`: 1, `0xCA316BEB`) | 0 (`0x0090C7F3`: 1, `0x2A316C43`) | |
| 3 `SetInput(record)` (`0x0090C8C4`) | 3 (the same function) | wraps the record in a file object (class `0x0A32C0DA`, IID `0x8A32C0AC`, its slot 3) and calls slot 4 with it |
| 4 `SetInput(file)` (`0x0090C69A`) | 4 `SetOutput(file)` (`0x0090C85B`) | ends an open read/write first; keeps the file (AddRef) only when it is open and its open mode allows reading (writer: writing); returns that test; else holds nothing |
| 5 `BeginRead(name, out)` (`0x0090CA2D`) | 5 `BeginWrite(name, arg)` (`0x0090CAB2`) | the codec by name; then `_GIMEX_open` / `_GIMEX_wopen` |
| 6 `BeginRead(codec, out)` (`0x0090CBF3`) | 6 `BeginWrite(codec, arg)` (`0x0090CB00`) | §1, then open; nothing when already open or no file |
| 7 `GetFrameInfo(frame)` (`0x0090C772`) | 7 `PutFramePixels(info, pixels, pitch)` (`0x0090C9AE`) | |
| 8 `GetFramePixels(info, pixels, pitch)` (`0x0090C7AB`) | 8 `EndWrite` (`0x0090C960`) | only when open |
| 9 `FreeGInfo` (`0x0090C7A6`) | | frees a frame description |
| 10 `EndRead` (`0x0090C724`) | | closes the library handle; "open" := 0 |
| 11 / 12 | 9 / 10 | deleting destructor (`0x0090CA10` / `0x0090CA95`); `FinalRelease` (`0x0090C669` / `0x0090C82A`): close the file if open, drop it (slot 4 with null), delete |

The writer's begin (`DoBeginWrite`, `0x0090C91F`) records whether the library opened the output.

## 3. The factory (`cGZGimexFactory`, IID `0x02FFBB60`) *[compat]*

Object: primary vtable `0x00ACE530`, the persist resource factory base at `+4` (vtable `0x00ACE4FC`, persistence.md
§4.2), count `+0x14`, dither settings `+0x18` = 1 and `+0x1C` = 1, a pixel scratch vector `+0x20`, a dither error
vector `+0x2C` (constructor `0x0090BC54`; destructor `0x0090BCB3` frees both vectors; deleting destructor
`0x0090BCEB`, thunks `0x0090BC9B`, `0x0090BCA3`, `0x0090BCAB`). `QueryInterface` (`0x0090B15E`): 1 and
`0x02FFBB60` → this, else the base. `Release` (`0x0090B187`): a count of 1 or less destroys (through the base
interface's slot 7).

| Slot | Address | Behaviour |
|---|---|---|
| 3 `Load(file, codec, buffer, type)` | `0x0090C56F` | reader (class `0x2A316C02`, IID `0xCA316BEB`) with `SetInput(file)`; §4 |
| 4 `Create(path, &buffer, type)` | `0x0090B7AC` | `CreateBuffer`, then slot 5; on failure the buffer is released and the output nulled |
| 5 `Load(path, buffer, type)` | `0x0090B732` | opens a `cRZFile(path)` for reading; slot 3 with codec `0x34`; closes |
| 6 `Save(file, codec, buffer, quality)` | `0x0090B245` | slot 7 with the buffer's whole area (buffer slot 12) |
| 7 `Save(file, codec, buffer, area, quality)` | `0x0090BE8E` | writer (class `0xCA316C55`, IID `0x2A316C43`) with `SetOutput(file)`; §6 |
| 8 `Save(path, buffer, quality)` | `0x0090B1A1` | slot 9 with the whole area |
| 9 `Save(path, buffer, area, quality)` | `0x0090B1D8` | opens a `cRZFile(path)` for writing; slot 7 with codec `0x33`; closes |
| 10 `Create(file, codec, &buffer, type)` | `0x0090B7EA` | `CreateBuffer`, then slot 3 (output nulled on failure) |
| 11 `Load(key, codec, buffer, type)` | `0x0090B89F` | the resource manager's record for `key` (its slot 37, read), its serial record interface, slot 13, then the record closed |
| 12 `Create(key, codec, &buffer, type)` | `0x0090B82C` | the same with slot 14 |
| 13 `Load(record, codec, buffer, type)` | `0x0090C5CB` | reader with `SetInput(record)`; §4 |
| 14 `Create(record, codec, &buffer, type)` | `0x0090B912` | `CreateBuffer`, then slot 13 |
| 15 `Save(key, segment, codec, buffer, quality)` | `0x0090B27F` | slot 16 with the whole area |
| 16 `Save(key, segment, codec, buffer, area, quality)` | `0x0090B954` | the record opened for writing (the given segment's slot 21 with access 2, or without a segment the resource manager's slot 37 for writing); its serial record; slot 18; closed |
| 17 `Save(record, codec, buffer, quality)` | `0x0090B2BC` | slot 18 with the whole area |
| 18 `Save(record, codec, buffer, area, quality)` | `0x0090BEE5` | writer with `SetOutput(record)`; §6 |

- `CreateBuffer(&out)` (`0x0090B6FC`): GZCOM class `0xC470D325` (IID `0x20732180`), or, when that fails, the
  graphics system's buffer creation (its slot 3). The graphics system (`0x0090B6CC`) and the resource manager
  (`0x0090B69C`) are cached on first use (`0x00B62598`, `0x00B62594`).
- As a persist resource factory (type `0x856DDBAC`, the persist buffer resource): `SerialRead(resource, record)`
  (`0x0090B31E`) loads the record into the resource's buffer with slot 13, codec `0x32` and the resource's colour
  type (`+0x24`); `SerialWrite` (`0x0090B378`) saves it with slot 17, codec 4 and quality 100. Other types fail.

## 4. Loading (`LoadWithReader`, `0x0090C443`) *[compat]*

1. `BeginRead(codec, &n)`; fail unless it succeeds with `n` ≠ 0.
2. The description of frame 0; the pixels (`width × height × bpp / 8` bytes into the scratch vector) read with
   pitch `width × bpp / 8`.
3. If the buffer is already initialised (its slot 39) it is shut down (slot 4); then `Init(width, height,
   (type, bits(type)))`; locked (`0x8080`); the pixels translated (§5); unlocked.
4. The description is freed and the read ended; the result is the last step's result.

## 5. Translation into the buffer (`TranslateGimexToGZBuffer`, `0x0090C1D4`) *[compat]*

False for a null buffer, info or pixel pointer. Rows of `width` pixels; the destination advances by the buffer's
stride per row.

**32-bit images**, by the buffer's colour type (the buffer is locked `0x8040` meanwhile):

| Type | Pixel (v = the 32-bit pixel) |
|---|---|
| 4 | `0x8000 \| (v >> 9) & 0xFC00 \| (v >> 6) & 0x3E0 \| (v >> 3) & 0x1F` (`0x0090B070`) |
| 5 | `(v >> 8) & 0xF800 \| (v >> 6) & 0x3E0 \| (v >> 3) & 0x1F` (`0x0090B0C0`): the green field gets only the top 5 bits of green, one bit too low, so green is halved *[quirk]* |
| 6 | `(v >> 9) & 0x7C00 \| (v >> 6) & 0x3E0 \| (v >> 3) & 0x1F \| (v >> 16) & 0x8000` (`0x0090B110`) |
| 7 | with the dither settings both 1 (the constructor's values): the dither below; otherwise the top nibble of each channel, `(v >> 16) & 0xF000 \| (v >> 12) & 0xF00 \| (v >> 8) & 0xF0 \| (v >> 4) & 0xF` (`0x0090B3C1`) |
| 8, 9 | copied |
| other | false |

**The 4444 dither** (`TranslateGimex24ToGZBufferARGB4444Dither`, `0x0090BF3C`). Two error rows `A` and `B` of
`width` packed entries (one byte lane per channel, lane c = byte c of the pixel), held in the factory's error
vector (resized to `2·width + 4` entries; one spare entry before each row). `A` := `0x48` in every lane, `B` := 0.
Rows are walked alternately left to right and right to left (direction d = +1, then −1), starting at x0 = 0 (then
`width` − 1, …). For each row:

1. Seed: e := the low nibble of byte 0 of the first pixel visited; `A[x0] += 7e`, `B[x0] += e`.
2. For each pixel i in the walk: for each lane c, `t_c = byte_c + ((A[i] >> (8c + 4)) & 0xF) − 8`, clamped to
   0…255. Output `(t3 >> 4) << 12 | (t2 >> 4) << 8 | (t1 >> 4) << 4 | t0 >> 4` (A, R, G, B). The lane errors
   q = the low nibbles `t_c & 0xF` packed the same way; then `A[i+d] += 7q`, `B[i+d] := q` (an assignment),
   `B[i] += 5q`, `B[i−d] += 3q` (whole-dword additions, so a lane sum above 255 carries into the next lane
   *[quirk]*).
3. After the row: `B[last] += 3q` once more with the last q; swap `A` and `B`; reverse d.

`B` is not cleared when it becomes the next row's error row; only the entries the walk assigns (`B[i+d] := q`) drop
their old value, so the start entry of each row keeps old error *[quirk]*. The result is true.

**8-bit images** (palettised), by the buffer's bits per pixel:

- 8: the index rows are copied (`0x0090B448`).
- 16: each index through a 256-entry cache of the palette converted to the buffer type (`0x0090B49C`): type 4 and 6
  `(R & 0xF8) << 7 | (G & 0xF8) << 2 | B >> 3` (`0x0090AE31`, no alpha bit), type 5 `(R & 0xF8) << 8 | (G & 0xFC) << 3
  | B >> 3` (`0x0090AE0E`), type 7 `0xF000 | (R & 0xF0) << 4 | G & 0xF0 | B >> 4` (`0x0090AE54`); any other 16-bit
  type gives `0xFFFF` for every pixel *[quirk]*. The destination advances by the stride rounded down to even.
- 32: the palette entry's first three bytes as one value p = `B | G << 8 | R << 16`: type 9 stores p (alpha 0
  *[quirk]*), any other 32-bit type `p << 8` *[quirk]* (`0x0090B5F1`).
- other: false.

## 6. Saving (`0x0090BD07`, from slots 7 and 18) *[compat]*

1. `BeginWrite(codec, 1)`; fail when it fails.
2. A Gimex description (`0x2A70` bytes, zeroed): size `0x2A70`, version `0x157`, width and height of the area
   (right − left, bottom − top), 32 bits per pixel (`+0x18`, `+0x1C`), the quality argument (`+0x430`), the data
   size `width × height × 4` (`+0x434`), channel bits 8 for red, green and blue (`+0x43C`…`+0x444`) and 8 for alpha
   only when the buffer type is 6, 7 or 9 (`+0x438`).
3. The area is converted to 32-bit pixels (`0x0090BA37`, buffer locked `0x8040`, rows from the area's top-left with
   the buffer stride): type 4 X1R5G5B5 → each 5-bit channel `c · 255 / 31` (rounded down), alpha `0xFF`
   (`0x0090AE80`); type 5 R5G6B5 → 5- and 6-bit channels scaled the same way (`/31`, `/63`) (`0x0090AF20`); type 6
   → as type 4 with alpha `0xFF` when bit 15 is set, else 0 (`0x0090AFC0`); type 7 → `((p & 0xF000) >> 4) \| (p &
   0xF0)` with the source advancing **one byte** per pixel, which garbles the image *[quirk]* (`0x0090B408`); types 8
   and 9 copied; any other type fails.
4. `PutFramePixels(description, pixels, width × 4)`; then `EndWrite`; the writer is released. The result is the put's
   result.

## 7. Function inventory

`cGZGimexFactory`: `0x0090AE0E`, `0x0090AE31`, `0x0090AE54`, `0x0090AE80`, `0x0090AF20`, `0x0090AFC0`,
`0x0090B070`, `0x0090B0C0`, `0x0090B110`, `0x0090B15E`, `0x0090B187`, `0x0090B1A1`, `0x0090B1D8`, `0x0090B245`,
`0x0090B27F`, `0x0090B2BC`, `0x0090B31E`, `0x0090B378`, `0x0090B3C1`, `0x0090B408`, `0x0090B448`, `0x0090B49C`,
`0x0090B5F1`, `0x0090B69C`, `0x0090B6CC`, `0x0090B6FC`, `0x0090B732`, `0x0090B7AC`, `0x0090B7EA`, `0x0090B82C`,
`0x0090B89F`, `0x0090B912`, `0x0090B954`, `0x0090BA37`, `0x0090BC54`, `0x0090BC9B`, `0x0090BCA3`, `0x0090BCAB`,
`0x0090BCB3`, `0x0090BCEB`, `0x0090BD07`, `0x0090BE8E`, `0x0090BEE5`, `0x0090BF3C`, `0x0090C1D4`, `0x0090C443`,
`0x0090C56F`, `0x0090C5CB`.

`cGZGimexReader`: `0x0090C62F`, `0x0090C654`, `0x0090C669`, `0x0090C69A`, `0x0090C701` (`_GIMEX_open` helper),
`0x0090C724`, `0x0090C772`, `0x0090C7A6`, `0x0090C7AB`, `0x0090C8C4`, `0x0090CA10`, `0x0090CA2D`, `0x0090CBF3`.
`cGZGimexWriter`: `0x0090C7F3`, `0x0090C815`, `0x0090C82A`, `0x0090C85B`, `0x0090C91F`, `0x0090C960`,
`0x0090C9AE`, `0x0090CA95`, `0x0090CAB2`, `0x0090CB00`.

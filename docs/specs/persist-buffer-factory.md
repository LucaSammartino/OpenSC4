# `cGZPersistBufferResourceFactory`: the persist-buffer image factory (behaviour spec)

Scope: the `cGZPersistBufferResourceFactory` owner as tools/coverage.py assigns it (9 functions, 2,294 bytes): the
factory of the image resource `cGZPersistBufferResource` (type `0x856DDBAC`, sc-resources.md §7). It loads and
saves a `cIGZBuffer` (a 2-D pixel buffer) as a record, reads a record into a resource, loads an image file, and
loads a Win32 bitmap resource of the executable.

Related: sc-resources.md §7 (the resource itself, its BMP/TGA input), persistence.md §4.2 (the factory base), §6
(the sibling `cGZBufferResource`, whose record forms these resemble) and §3 (record fields), span-render-4444.md
(colour types).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the owner was read in the decompile; `LoadFromSerialRecord`,
`SaveToSerialRecord`, `SerialRead` and `LoadFromFile` were read in the disassembly, and the record's interface
vtable `0x00AD9178`. Written 2026-10-24 (re-persistgraph7).

A **colour pair** is (colour type, bits per pixel); `0x0044A490` makes it from a colour type with the table of
persistence.md §6.1. Buffer slots used: 3 `Init(width, height, pair)`, 6 / 7 lock / unlock (flags `0x8040`,
`0x8080`, `0x8010`), 9 / 10 width / height, 15 the colour pair, 17 bytes per pixel, 22 set pixel, 29 copy from
another buffer, 30 pack an RGB triple, 34 the pixel pointer, 35 the row stride.

## 1. Identity *[compat]*

- `QueryInterface` (`0x00977215`; thunk from the second interface `0x009780A8`): IID `0x656DDC26` → the interface
  at `+0x10`, AddRef'd; anything else → the factory base (persistence.md §4.2).
- `SetColorType(type, bits)` (`0x00978097`): stores the pair at `+4` / `+8`; no function of this owner reads it.

## 2. The record forms (`LoadFromSerialRecord(out, record, option)`, `0x00977ADC`) *[compat]*

False when `out` or `record` is null. The `option` argument is not used. The record is read through its 32-bit
field getter (persistence.md §3.2), starting with a u32 **version**:

| Version | Body |
|---|---|
| 1 | u32 dataSize, u32 width, u32 height, u32 colourType, then dataSize raw bytes of pixels |
| 2 | the same with the pixels compressed (dataSize = compressed size); the codec of persistence.md §6.2 v2 |
| `0x856DDBAC` | u32 width, u32 height, then width × height u32 pixels (row-major, row 0 first) |
| anything else | nothing more is read and the call returns **true** with `out` untouched *[quirk]* |

**Versions 1 and 2.** A buffer is created through the graphics system and initialised at (width, height,
pair(colourType)); it is locked (`0x8040`). With `bpp` = bits / 8 the packed row is `width × bpp` bytes; the buffer
is "padded" when its stride differs from that.

- v1: the dataSize bytes are read straight into the buffer, or (padded) into a scratch copy from which `height`
  rows of `width × bpp` bytes are copied. dataSize is not checked against the buffer size *[quirk]*. In the padded
  copy the **source** advances by the stride and the destination by the packed row (the same swap as
  persistence.md §6.2 v1) *[quirk]*.
- v2: the compressed bytes are read into a scratch block and decompressed into the buffer (or a scratch copy when
  padded); the load fails unless the output is exactly `width × height × bpp` bytes; padded rows are copied packed
  source to stride-spaced destination.
- The buffer is unlocked and converted to its own colour type (`ConvertBufferFromFormatToFormat`, `0x00931CF0`);
  that result is the call's result.
- A failed read of dataSize, width, height or colourType, or a failed buffer creation, reaches the stride query
  with no buffer *[quirk: a null dereference]*; *[choice]* FreeCity4 fails the load.

**Version `0x856DDBAC`.** A buffer is created, the pixels are read into a scratch array, the buffer is initialised
at (width, height, pair(3)) (16 bits), locked (`0x8080`) and each pixel is stored with the buffer's set-pixel
(which converts the 32-bit value to the buffer format), row by row; then unlocked. A failed lock skips the pixels
but the call still succeeds *[quirk]*.

On success `out` receives the buffer (the creation reference); on failure the buffer is released and `out` is left
unchanged.

## 3. Saving (`SaveToSerialRecord(buffer, record, compress)`, `0x00977E93`) *[compat]*

False when `buffer` or `record` is null, or the buffer cannot be locked (`0x8040`). The pixels are packed (rows
copied without the stride padding when the buffer is padded). With `compress` they are compressed into a block of
`size × 15 / 10 + 1,024` bytes; if the compressor fails (its size result is then 0) the record falls back to
version 1 with that size *[quirk: a version-1 record with 0 data bytes]*. Then: u32 version (2 compressed, else 1),
u32 dataSize, u32 width, u32 height, u32 colourType (the buffer's pair, first word), the data bytes raw. The first
failed write stops the sequence; the result is the last write's result. The buffer is unlocked and the scratch
blocks freed.

## 4. Reading into a resource (`SerialRead(resource, record)`, `0x009791B1`) *[compat]*

- False unless the resource's type is `0x856DDBAC` and its buffer interface (`+0x18`, slot 3) yields a target
  buffer.
- A temporary `cGZPersistBufferResource` with the resource's colour pair (`+0x24`, `+0x28`) loads the record
  (`LoadFromRecord`, `0x00979171`, sc-resources.md §7); its result is returned when it is not true.
- On success the target is initialised at the loaded image's width, height and colour pair, locked (`0x8010`),
  filled with a copy of the whole loaded image (slot 29, with the image's bounds), and unlocked. The call returns
  true once the load succeeded, also when the target's `Init` or lock fails *[quirk]* (false only when the loaded
  resource has no buffer).

## 5. Loading a file (`LoadFromFile(path, out, a, b, c, d)`, `0x009790ED`) *[compat]*

Creates a `cGZPersistBufferResource` (`0x38` bytes, colour pair(1)) and calls its `LoadFromFile(path, a, b, c, d)`
(`0x0097878C`, sc-resources.md §7.2); on success the resource is asked for the buffer interface `0x20732180` into
`out`, and that answer is the result. The resource is never released *[quirk: a leak]*, and a failed allocation is
not checked.

## 6. Loading a bitmap resource (`LoadFromResourceId(id, out, …, colourType, bits)`, `0x00977A06`) *[compat]*

1. `LoadImageA(the executable's module, MAKEINTRESOURCE(id & 0xFFFF), IMAGE_BITMAP, 0, 0, LR_CREATEDIBSECTION |
   LR_DEFAULTSIZE)`; no image → false.
2. `GetObjectA` for the DIB section; if it fails the call returns **true** with nothing created *[quirk]*.
3. Only a 24-bit bitmap loaded with `bits` = 16 is accepted (else false). A buffer is created through GZCOM (class
   `0xC470D325`, IID `0x20732180`) into `out`, AddRef'd, and initialised at (width, height, (colourType, 16)).
4. The pixels are converted (`0x0097723F`): the buffer is locked (`0x8040`); the bitmap's rows are bottom-up, so
   source row 0 fills the buffer's last row; each pixel's three bytes (blue, green, red) become one 16-bit value
   through the buffer's RGB packer (slot 30, called with red, green, blue); each source row is followed by
   `width mod 4` padding bytes (the DIB row padding for 24-bit rows); then unlocked.
5. A failed `Init` or conversion releases the buffer once (leaving `out` pointing at it) and returns false. The
   bitmap object is deleted in every case after step 1. On success `out` holds two references (creation and the
   AddRef) *[quirk]*.

## 7. Function inventory

`0x00977215`, `0x0097723F`, `0x00977A06`, `0x00977ADC`, `0x00977E93`, `0x00978097`, `0x009780A8`, `0x009790ED`,
`0x009791B1`.

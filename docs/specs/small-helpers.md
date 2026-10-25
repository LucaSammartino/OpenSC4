# Small free helpers: temp names, record export, resource error, spawning, buffer conversion (behaviour spec)

Scope: the free-function owners `fn:GetNewTempFilePath` (1 function, 440 bytes), `fn:SC4ExportRecordToFile` (2, 348),
`fn:SC4DisplayResourceError` (1, 397), `fn:RZSpawnWait` (1, 280), `fn:ConvertBufferFromFormatToFormat` (1, 509),
`fn:S3DTransformLegacyLoad` (1, 322), the two `fn:SC4CreateWindowFromScript` overloads that ui-windows.md §5.1 only
lists, and the power-of-ten wrapper of `fn:RZStrtodEnglishA`, as tools/coverage.py assigns them.

Related: framework-rz-files.md (`cRZFile`, its `Open` modes §2.3), data-formats.md (the number parsers),
language-utility.md (language IDs), persistence.md §6 and persist-buffer-factory.md (buffer conversion callers),
lot-records.md (the legacy transform), ui-windows.md §5.1 and ui-helpers.md (scripted windows), ui-script-service.md
(the UI script service).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile and in the disassembly; the strings were read
from the data section. Written 2026-10-24 (re-smallhelpers7).

## 1. `GetNewTempFilePath(dir, ext, out)` (`0x00818F00`) *[compat]*

Reserves a new file name. `n` starts at the current time (`time(NULL)`, seconds). Each try sets
`out := dir + decimal(n) + ext` (`%d`) and opens it with `cRZFile::Open(write, create new, share read)`: it succeeds
only when no such file exists, and then leaves an **empty file** behind (closed) and returns true. Otherwise `n`
decreases by 1. After 8,192 failed tries `out` is emptied and the result is false.

## 2. Exporting a record to a file (`fn:SC4ExportRecordToFile`) *[compat]*

- `ExportRecord(segment, key, path)` (`0x005DBC40`): notes whether `path` already exists, then opens it with
  `Open(write, open always, share read)`. On success: the record's size (segment slot 20 with `key`); a buffer of that
  size; the record's bytes (segment slot 28, `(key, buffer, &size)`); the file truncated to 0 and the bytes written.
  The result is the byte count, or −1 when the size, the read or the write failed. The file is closed; on failure a
  file that did not exist before is deleted. A failed open returns −1 and leaves nothing.
- `SC4ExportRecordToFile(key, path)` (`0x005DBDC0`): the resource manager's segment holding `key` (its slot 32); none
  → −1; else `ExportRecord(segment, key, path)` and the segment is released.

## 3. `SC4DisplayResourceError(language)` (`0x005F8D40`) *[compat]*

A message box (title "SimCity 4", OK button) saying that the game data cannot be found, in the language given by
its ID (language-utility.md): 3 French, 4 German, 5 Italian, 6 Spanish, 7 Dutch, 8 Danish, 9 Swedish, 10 Norwegian,
11 Finnish, 14 Portuguese, 15 Japanese, 16 Polish, 17 Simplified Chinese, 18 Traditional Chinese, 19 Thai, 20 Korean
each have their own text (strings `0x00AA5E18`…`0x00AA6428`, in the language's code page); every other ID, English
(1, 2) included, gets "Unable to find game data. This is likely an installation problem or operating system
problem."

## 4. `RZSpawnWait(program, arg1, …, NULL)` (`0x0091234F`) *[compat]*

Runs `program` and waits for it (`_spawnv(_P_WAIT, program, argv)`), returning its result; a null program gives −1.
Every argv string is wrapped in double quotes. Up to 32 arguments are taken from the null-terminated list:

- when `arg1` equals `program` (`strcmp`), argv is `"arg1"`, `"arg2"`, …;
- otherwise argv[0] is `"program"` and **`arg1` is lost**: it is appended to the same string after its terminating
  NUL *[quirk]*; argv continues with `"arg2"`, …;
- with no argument at all (`arg1` null), argv is empty (its first entry is overwritten by the null) *[quirk]*.

## 5. `ConvertBufferFromFormatToFormat(buffer, colorType)` (`0x00931CF0`) *[compat]*

Changes a buffer's colour type in place, keeping its pixels (persistence.md §6, the buffer-resource loaders):

1. Already of that type → true.
2. The width, height and transparent colour (slot 26) are noted; the transparency is removed (slot 27) and the
   transparent colour split into red, green, blue (slot 31).
3. A temporary buffer (GZCOM class `0xC470D325`, IID `0x20732180`; none → false) is initialised with the same size
   and the **new** type (bits per pixel from the type, persistence.md §6.1) and receives the buffer through `Blt` of
   the whole area under the `0x8010` lock (the format conversion happens here).
4. The buffer is shut down (slot 4), initialised again with the new type, and the temporary is copied back into it.
5. A transparent colour is set again from its red, green, blue in the new format (slots 30, 23); its alpha is not
   kept *[inferred]*.

Failures of either `Init` release the temporary and return false; after step 4 fails the buffer is left shut down
*[quirk]*.

## 6. `S3DTransformLegacyLoad(stream, xform)` (`0x005DA930`) *[compat]*

Reads the version-4 transform of lot-records.md: nine `f32` (the 3 × 3 matrix, `0x005DA650`) into `+0x04`…`+0x24`,
then three `f32` into the translation `+0x28`…`+0x30`, then one `f32` scale into `+0x34`. The flag byte `+1` gets
`0x4` (matrix) and `0x1` (translation); `0x2` is set when the scale is not exactly 1.0 and cleared when it is. The
byte `+0` is incremented after each part and finally **replaced** by the low byte of a `u32` read last (stream
slot 9). Values a failed read does not write keep the global default `0x00B23044`.

## 7. The other `SC4CreateWindowFromScript` overloads *[compat]*

Both look up the UI script service (service `0x5A356E15`, IID `0xFA3562FA`) like the first overload (ui-windows.md
§5.1) and return false without it. They call its slot 3, which creates the window, queries it for an interface and
releases it (ui-script-service.md):

- `(instance, parent, id, iid, out)` (`0x005F93F0`): with the key `{0, 0x96A006B0, instance}`, the UI script group.
- `(key, parent, id, iid, out)` (`0x005F9480`): all five arguments forwarded unchanged.

The result is the service's boolean.

## 8. The power of ten as a float (`0x0091040D`) *[compat]*

`(float)Pow10(base, n)` for the decimal parser of data-formats.md: the 32-bit wrapping integer power `0x009103DA`,
converted to the x87 floating value returned.

## 9. Function inventory

`fn:GetNewTempFilePath`: `0x00818F00`. `fn:SC4ExportRecordToFile`: `0x005DBC40`, `0x005DBDC0`.
`fn:SC4DisplayResourceError`: `0x005F8D40`. `fn:RZSpawnWait`: `0x0091234F`.
`fn:ConvertBufferFromFormatToFormat`: `0x00931CF0`. `fn:S3DTransformLegacyLoad`: `0x005DA930`.
`fn:SC4CreateWindowFromScript`: `0x005F93F0`, `0x005F9480` (and `0x005F9390`, ui-windows.md).
`fn:RZStrtodEnglishA`: `0x0091040D` (and `0x009103DA`, `0x00910426`, data-formats.md).

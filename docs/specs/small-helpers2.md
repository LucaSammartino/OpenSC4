# Small free helpers 2: director getters, number and hex text, patterns, save-game records, Gimex I/O (behaviour spec)

Scope: 68 free-function owners (`fn:` in tools/coverage.py), one to three functions each, listed in the inventory.
Left to their areas: the text-encoding character helpers (`GetCharLength`, `GetComposedCharLength` and the others of
text-edit-encoding.md), the effects-parser switches (`ParseDrawOptions`, `ParseFloatRange`, `ParseRangedFloat`,
`ParseRangedVector3`), the audio factories (`CreatecHitMan`, `CreatecHitDebugInfo`, the radio and Hit callbacks) and
`GZCOM` (system-service-pointer.md).

Related: framework-misc2.md §2 (allocators), app-directors.md and renderer-gdrivers-director.md (the directors),
data-formats.md (number parsing in exemplar text), foundations.md §2.7 and lot-records.md (serializable fields),
save-records.md (the version record), region-model.md (opening saved cities), legacy-control.md §9 (window
parameters), network-rules.md (network names, `[AllowedOverrides]`), gimex.md (the image library), lua-host.md
(CRC-24).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the save-game, notification, sort and
director bodies were read in the disassembly. Written 2026-10-24 (re-smallhelpers7b).

## 1. Director getters *[compat]*

Each returns a static director object, constructed on the first call (a guard bit) and registered for destruction at
exit (`__cxa_atexit`).

| Function (address) | Director | Object |
|---|---|---|
| `RZGetCOMDllDirector` (`0x0087B212`), `GZDllGetGZCOMDirector` (`0x0090DDFD`, the same body) | `cGZCOMDirector` | `0x00B540B8` |
| `SC4GetCOMDirector` (`0x00460F80`) | `cSC4COMDirector` | `0x00B214F8` |
| `GetSC4GDriversCOMDirector` (`0x00466440`) | `cSC4GDriversCOMDirector` | `0x00B21598` |
| `SC4AutomataGetCOMDirector` (`0x00514C30`) | `cSC4AutomataCOMDirector` | `0x00B22778` |
| `GetSC4ResourceLibCOMDirector` (`0x00664B60`) | `cSC4ResourceLibCOMDirector` | `0x00B468A0` |
| `GetGZDrawCOMDllDirector` (`0x00825C40`) | `cGZDrawCOMDirector` | `0x00B50840` |
| `GetFontComDirector` (`0x00896012`) | `cFontCOMDirector` | `0x00B543C8` |
| `GetGZUIScriptCOMDirector` (`0x0094B053`) | `cGZUIScriptCOMDirector` | `0x00B63538` |
| `GetGZResourceCOMDirectorW32` (`0x00975037`) | `cGZResourceCOMDirectorW32` (framework-misc2.md §8) | `0x00B63640` |
| `GetGZResManCOMDirector` (`0x00975363`) | `cGZResManCOMDirector` | `0x00B63690` |
| `GetGZWinCOMDllDirector` (`0x00998E48`) | `cGZWinCOMDirector` | `0x00BAC008` |
| `GetGZWin32COMDllDirector` (`0x009DA5FE`) | `cGZWin32COMDirector` | `0x00BAC0E8` |

## 2. Numbers and hexadecimal text *[compat]*

- `RZAtoUint32HexA(s)` (`0x0090FCEF`): skips white space and an optional `0x` / `0X`, takes at most 8 hexadecimal
  digits up to the first other character and returns their value (0 with none).
- `RZConvertToUint32A(s)` (`0x0090FE93`): a `0x` / `0X` prefix → `strtoul` base 16. Otherwise base 16 when a letter
  `a`–`f` / `A`–`F` appears among the leading digits (the scan stops at the first character that is neither), else
  base 10.
- `RZAtoi64A(s)` (`0x0090FC6F`): white space, one optional sign, decimal digits in 64-bit arithmetic (wrapping),
  negated for `-`.
- `RZAtofA(s)` (`0x0090FC5B`): the C runtime's `strtod` (locale-dependent). `RZAtofEnglishA(s)` (`0x00910991`): the
  game's own decimal parser (data-formats.md, `RZStrtodEnglishA`).
- `ConvertBinaryDataToASCIIArray(data, n, out)` (`0x0090FEF4`): two uppercase hexadecimal digits per byte, then the
  character **`'0'`** (not a NUL) after the last pair *[quirk]*.
- `ConvertASCIIArrayToBinaryData(text, n, out)` (`0x0090FF33`): pairs of hexadecimal digits (either case) to bytes over
  `n` characters; any other character counts as `0` and makes the result false, else true.
- `ThreadSafeIncrementReturn(p, n)` (`0x00933015`): atomic `*p += n`, returns the new value.
- `memset32(p, v, n)` (`0x00910003`): `n` dwords; `memset8(p, v, n)` (`0x0090FFB1`): the C `memset`.

## 3. Strings, patterns, files *[compat]*

- `MatchPattern(text, pattern)` (`0x00918103`): `?` matches one character, `*` any run (tried recursively), a pattern
  ending in `*` matches the rest at once; other characters must be equal (case-sensitive).
- `WildcardMatch(text, pattern, caseSensitive)` (`0x00918EE2`): copies both, lower-cases them when not case-sensitive,
  then `MatchPattern`.
- `IsFileNameCharAllowed(c)` (`0x009180D4`): false for `< > : \ / " | * ?`, else true.
- `SC4LimitStringLength(s, max)` (`0x005F88D0`): cuts the string at the last character boundary (`GetCharLength`)
  that keeps it within `max` bytes; true when it was cut.
- `AppendNetworkName(type)` (`0x0063E240`): appends the network name of network-rules.md (`Road`, `Rail`, `Highway`,
  `Street`, `Pipe`, `PowerLine`, `Avenue`, `Subway`, `LightRail`, `Monorail`, `OneWayRoad`, `DirtRoad`,
  `GroundHighway` for 0–12); other values append nothing.
- `FileBrowserSortFunction(a, b)` (`0x0066E3A0`): true when `b`'s name sorts before `a`'s (bytewise, the shorter first
  on a common prefix), so the list sorts in descending order *[inferred from the argument swap]*.
- `RZGetCurrentAppPath(out)` (`0x00912467`): the executable's path from `GetModuleFileNameW` (looked up once,
  converted from UTF-16), else from `GetModuleFileNameA`.
- `RZSetEnvironmentVariable(name, value)` (`0x0090FF9C`): `SetEnvironmentVariableA`, true on success.
- `ReadFileIntoString(path, out, max)` (`0x008B19D0`): reads `min(size, max)` bytes of the file into `out`; on any
  failure `out` is emptied; returns success.
- `RectContainsInclusive(rect, x, y)` (`0x0043FDE0`): `left ≤ x ≤ right` and `top ≤ y ≤ bottom`.
- `QueryDebugOutput(format, …)` (`0x004C3750`): formats into 511 characters and writes the text to the debug console
  (`0x00B21850`, its slot 14) when one exists. `WriteConnectionInfo(name)` (`0x004C6A60`): for a valid neighbour
  connection cell with a connection, prints `-> <name>` that way (a debugging aid).
- `__export__GetMemoryReport` (`0x00A6D82C`): returns the global `0x00BB88AC`.

## 4. Save games and records *[compat]*

- `SC4OpenSaveGame(path, writable, checkVersion)` (`0x005DBFE0`): a new database segment (`0x1DC` bytes); after its
  `Init` the path is set and it is opened (read-only unless `writable`). With `checkVersion` the application version
  (below) must be 62–65 (region-model.md), else the segment is shut down; returns the segment, or null after a
  failure (released).
- `SC4CloseSaveGame(seg)` (`0x005DBC10`): when open, closes it (slots 8, 5) and releases it; true; else false.
- `SC4ReadAppLoadSaveVersion(seg)` (`0x005DBF30`): the 4-byte record type `0xEA3FF4F5`, group `0xEA3FF506`, instance
  `0xEA3FF4F5` (foundations.md); a missing record means 65 (`0x41`), an existing record that cannot be read gives 0.
  `SC4WriteAppLoadSaveVersion(seg)` (`0x005DBFA0`) writes 65 there.
- `SC4ReadStringRecord(seg, key, string)` / `SC4WriteStringRecord` (`0x005DBE20`, `0x005DBEA0`): open the record
  (segment slot 21, read 1 / write 3), read or write the string through the record (its `cIGZString` text slots), close
  and release; true on success.
- `SC4ReadGZSerializable` (`0x005D9730` typed, `0x005D96C0` inner) and `SC4WriteGZSerializable` (`0x005D99A0`): the
  serializable-field helpers of lot-records.md (read ownership) and foundations.md §2.7.

## 5. Window parameters (legacy-control.md §9) *[compat]*

`SetResourceKeyParam(id, key)` (`0x0094C1A4`), `SetGZWinImage(key, n)` (`0x0094C1DA`, parameter `0xDAA3B860 + n`, an
8-byte value) and `SetGZWinFont` (`0x0094C222`, `0x0094C255`, parameter `0xFAA3BE85 + n`) store a new variant as a
window parameter (window slot 114). `GetResourceKeyParam(win, id, out)` (`0x0095BAD7`) copies the stored 12 bytes into
`out` when the parameter exists and its value's slot 4 gives 12 (the key `SetResourceKeyParam` stores), true then;
`GetGZWinImage(win, &a, &b, n)` (`0x0095BB29`) reads parameter `0xDAA3B860 + n` and, when its type has the array
bit, gives its first two dwords.

## 6. Simulation helpers *[compat]*

- `SafeGetAltitude(x, z)` (`0x004C38D0`): clamps `x` and `z` into the city's extent (cells × cell size, negatives to
  0) and asks the terrain for the altitude there.
- `GetPropTypeIndex(type)` (`0x005EF2E0`): the index of `type` in the global prop-type list, appended when missing
  (16-bit result).
- `SC4NetworkLoadOverrideMatrix(ini, matrix)` (`0x0063D630`): the section `AllowedOverrides` read with
  `LoadOverrideCallback` (network-rules.md §8.1). `LoadVerifiersCallback` (`0x0063D7E0`) is a hash-multimap insert
  (key `u32`, the entry copied) used by the tile verifier loader (network-rules.md §9).
- `SC4GameDelay(ms, flag)` (`0x005F9140`): a `cSC4WinGameDelay` window (`0x100` bytes, duration `ms`, mode
  `flag == 0`) is added to the window manager (slot 41), run (its slot 5) and released.
- `SC4FlushInputMessageQueue()` (`0x005F8610`): the window manager's slot 19.
- `SC4GetResourceIDFromString(s)` (`0x005F8BB0`): `CRC24(s) | 0xFF000000` (lua-host.md).
- `sNotify<cSC4Advisor>(post, type, data1)` (`0x0040DA10`, on an advisor): a standard message with that type, the
  advisor as IGZUnknown (its `QueryInterface(1)`; failure → nothing) and data1, then `MessagePost` (normal priority)
  or `MessageSend`.

## 7. Small library and container bodies *[compat]*

- The Gimex library's I/O callbacks over a GZ file (gimex.md): `_galloc` / `_gfree` (`0x0090CD3D`, `0x0090CD4A`, the
  global heap), `_glen` (`0x0090CDCC`, file slot 7), `_gread` / `_gwrite` (`0x0090CD65`, `0x0090CD8C`: a negative count
  gives 0, else slot 15 / 17 and the count done), `_gseek` (`0x0090CDB3`, slot 12; true when it returned the file).
- `GlobalHeapAlloc(n)` / `GlobalHeapFree(p)` (`0x005E55E0`, `0x005E5620`): the global fast allocator under its lock
  (framework-misc2.md §2.1; the lock object `0x00B230E0`).
- `operator*` (`0x0082CDF0`): a vector of 24-byte elements growing on insert (new capacity `size + max(size, n)`).
- `operator.new` (`0x00648F00`): the shared `return true` body.
- `create_cursor` (`0x0097E1D5`): the cursor resource's constructor (framework-misc2.md §7): a persistent resource with
  the cursor created through GZCOM (class `0x286A98A8`, IID `0x834C471E`) at `+0x1C`.

## 8. Function inventory

`fn:AppendNetworkName` `0x0063E240`; `fn:ConvertASCIIArrayToBinaryData` `0x0090FF33`;
`fn:ConvertBinaryDataToASCIIArray` `0x0090FEF4`; `fn:FileBrowserSortFunction` `0x0066E3A0`;
`fn:GZDllGetGZCOMDirector` `0x0090DDFD`; `fn:GetFontComDirector` `0x00896012`; `fn:GetGZDrawCOMDllDirector`
`0x00825C40`; `fn:GetGZResManCOMDirector` `0x00975363`; `fn:GetGZResourceCOMDirectorW32` `0x00975037`;
`fn:GetGZUIScriptCOMDirector` `0x0094B053`; `fn:GetGZWin32COMDllDirector` `0x009DA5FE`; `fn:GetGZWinCOMDllDirector`
`0x00998E48`; `fn:GetGZWinImage` `0x0095BB29`; `fn:GetPropTypeIndex` `0x005EF2E0`; `fn:GetResourceKeyParam`
`0x0095BAD7`; `fn:GetSC4GDriversCOMDirector` `0x00466440`; `fn:GetSC4ResourceLibCOMDirector` `0x00664B60`;
`fn:GlobalHeapAlloc` `0x005E55E0`; `fn:GlobalHeapFree` `0x005E5620`; `fn:IsFileNameCharAllowed` `0x009180D4`;
`fn:LoadVerifiersCallback` `0x0063D7E0`; `fn:MatchPattern` `0x00918103`; `fn:QueryDebugOutput` `0x004C3750`;
`fn:RZAtoUint32HexA` `0x0090FCEF`; `fn:RZAtofA` `0x0090FC5B`; `fn:RZAtofEnglishA` `0x00910991`; `fn:RZAtoi64A`
`0x0090FC6F`; `fn:RZConvertToUint32A` `0x0090FE93`; `fn:RZGetCOMDllDirector` `0x0087B212`; `fn:RZGetCurrentAppPath`
`0x00912467`; `fn:RZSetEnvironmentVariable` `0x0090FF9C`; `fn:ReadFileIntoString` `0x008B19D0`;
`fn:RectContainsInclusive` `0x0043FDE0`; `fn:SC4AutomataGetCOMDirector` `0x00514C30`; `fn:SC4CloseSaveGame`
`0x005DBC10`; `fn:SC4FlushInputMessageQueue` `0x005F8610`; `fn:SC4GameDelay` `0x005F9140`; `fn:SC4GetCOMDirector`
`0x00460F80`; `fn:SC4GetResourceIDFromString` `0x005F8BB0`; `fn:SC4LimitStringLength` `0x005F88D0`;
`fn:SC4NetworkLoadOverrideMatrix` `0x0063D630`; `fn:SC4OpenSaveGame` `0x005DBFE0`; `fn:SC4ReadAppLoadSaveVersion`
`0x005DBF30`; `fn:SC4ReadGZSerializable` `0x005D96C0`, `0x005D9730`; `fn:SC4ReadStringRecord` `0x005DBE20`;
`fn:SC4WriteAppLoadSaveVersion` `0x005DBFA0`; `fn:SC4WriteGZSerializable` `0x005D99A0`; `fn:SC4WriteStringRecord`
`0x005DBEA0`; `fn:SafeGetAltitude` `0x004C38D0`; `fn:SetGZWinFont` `0x0094C222`, `0x0094C255`; `fn:SetGZWinImage`
`0x0094C1DA`; `fn:SetResourceKeyParam` `0x0094C1A4`; `fn:ThreadSafeIncrementReturn` `0x00933015`; `fn:WildcardMatch`
`0x00918EE2`; `fn:WriteConnectionInfo` `0x004C6A60`; `fn:__export__GetMemoryReport` `0x00A6D82C`; `fn:_galloc`
`0x0090CD3D`; `fn:_gfree` `0x0090CD4A`; `fn:_glen` `0x0090CDCC`; `fn:_gread` `0x0090CD65`; `fn:_gseek` `0x0090CDB3`;
`fn:_gwrite` `0x0090CD8C`; `fn:create_cursor` `0x0097E1D5`; `fn:memset32` `0x00910003`; `fn:memset8` `0x0090FFB1`;
`fn:operator*` `0x0082CDF0`; `fn:operator.new` `0x00648F00`; `fn:sNotify` `0x0040DA10`.

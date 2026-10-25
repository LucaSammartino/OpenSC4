# Small framework owners: registry, extension map, key converter, text file, file buffer (behaviour spec)

Scope: the owners `cGZRegistry` (22 functions, 1,931 bytes), `cGZRESIDtoExtensionMap` (11, 607),
`cGZFileNameToResKeyConverterDefault` (2, 606), `cRZTextFile` (7, 989) and `fn:GetTrueName` (13, 775) as
tools/coverage.py assigns them.

Related: framework-gz-services.md and framework-rz-files.md (system services, `cRZFile`, `cIGZFile`),
sc-resources.md §11 (`cRZTextFile` open and `ReadLine`), graphics-rules.md §6 (`cRZTextFile::WriteLine`),
output-streams.md §4 (`cRZOFStream`, which owns a C++ file stream), application.md §9 (the directory segments and
`[DBSegmentDirectoryFiles]`), audio.md §10 (the Hit directory segments).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the five owners was read in the decompile; the registry's vtables
(`0x00ACFDAC`, `0x00ACFD60`, `0x00ACFD94`), its `Remove`, its separator strings, the converter's `FileNameToKey`
and the extension map's slots were read in the disassembly. Written 2026-10-24 (re-fwmisc7).

## 1. The registry (`cGZRegistry`, IID `0xA237613C`) *[compat]*

A system service (service ID `0x25076B9E`, priority 10,000,010; the base at `+4`, framework-gz-services.md) holding a
multimap from a key string to (name, value) string pairs, ordered by key (`+0x20`). Entries are written as URLs:
`key://name/value` (separators `://` and `/`, static strings at `0x00B629C0`, `0x00B629D4`).

| Slot | Address | Behaviour |
|---|---|---|
| 0 `QueryInterface` | `0x00921BB0` (thunks `0x00921FC2`, `0x00921FCA`) | `0xA237613C` → this; else the system service's |
| 3 `Add(entry)` | `0x0092212C` | key := the text before the first `://`; of the rest, name := the text before the first `/`; of what follows, value := the text before the next `/` (or to the end); the entry (key, (name, value)) is inserted after any equal keys. Missing separators are not tested *[quirk: the search's "not found" value is used as a position]* |
| 4 `Remove(key)` | `0x00921D97` | **ignores its argument**: it looks up the empty key and erases what the lookup returns, even when nothing matched *[quirk: undefined natively when there is no empty key]*; true. *[choice]* FreeCity4 removes nothing |
| 5 `Get(key, name, out)` | `0x00921DD8` | from the first entry with the key to the **end of the map**, the first entry whose name equals `name` (case-sensitive) gives `out` its value; true. Later keys' entries are tested too *[quirk]*; none → false |
| 6 `Format(key, name, value, out)` | `0x00921C95` | `out` := key + `://` + name + `/` + value; a null key → false |
| 7 `Enumerate(key, callback, context)` | `0x00921EB2` | false when the callback or the context is null; else from the first entry with the key to the end, each entry whose key equals `key` (case-sensitive) is passed as `callback(name, value, context)`; true |

Constructor `0x00922088`; destructor body `0x00921F9B`, deleting destructor `0x009220C6` (thunk `0x00921FD2`). The
rest is the map's code: `0x00921C12`, `0x00921C25` (erase one entry), `0x00921C60` (free a subtree), `0x00921D27`,
`0x00921D49` (make / copy an entry), `0x00921D6E`, `0x00921F63` (clear and destroy), `0x00921F7A` (new node),
`0x00921FF3` (link a node), `0x009220E2` (insert after equal keys).

## 2. The resource-type to extension map (`cGZRESIDtoExtensionMap`, IID `0xA56C8CFD`) *[compat]*

An ordered multimap from a resource type (u32) to a file extension, used by the directory segment (class
`0x090F8619`, `cGZDBSegmentDirectoryFiles`) that presents loose files as resources (application.md §9, audio.md §10).

| Slot | Address | Behaviour |
|---|---|---|
| 0 / 2 | `0x0097944E` / `0x00979473` | IIDs 1 and `0xA56C8CFD`; a count of 1 or less destroys (slot 8) |
| 3 `Add(type, ext)` | `0x0097B612` | inserts (type, ext) after any entries of the same type (duplicates are kept); true |
| 4 `ExtensionToRESID(ext, &type)` | `0x00979B8B` | the first entry, in type order, whose extension equals `ext` ignoring case; `ext` may carry a leading `.` (it is then compared without it); true with the type, else false |
| 5 `RESIDToExtension(&type, out)` | `0x00979C21` | the first entry for the type: `out` := its extension, true; else false |
| 6 `EraseAll` | `0x0097B2C7` (body `0x0066F8A0`) | empties the map |
| 7 `Copy(other)` | `0x00979C5A` | `EraseAll`, then `Add` of each of the other's entries |
| 8 | `0x0097B3E7` | deleting destructor (entry helper `0x0097AE66`) |

`0x0097B4E5`, named after the map, is the directory segment's destructor body: it restores the segment's vtables,
shuts the segment down, destroys its callback timer, its extension map and its path string, then the segment base.

## 3. The default file-name converter (`cGZFileNameToResKeyConverterDefault`) *[compat]*

The directory segment's default conversion between file names and resource keys:

- `FileNameToKey(name, &key)` (`0x0097A1F8`): splits the name at its last `.`, looks for runs of 8 or 16
  hexadecimal digits at the start or the end of the base name (by the converter's two flags `+8`, `+9`) and parses
  them, but **never stores** the parsed values and **always returns false** *[quirk]*. The key is left unchanged.
- `IsFileNameValid(name)` (`0x0097942D`): `FileNameToKey` with a zeroed key; therefore always false.
- `KeyToFileName` is the shared false-returning stub (`0x0093877E`).

So with the default converter no loose file maps to a key by its name; the segment's other mechanisms decide
*[inferred]*.

## 4. The text file (`cRZTextFile`) *[compat]*

A `cRZFile` (framework-rz-files.md) with a text mode (`+0x80`: 0 bytes, 2 UTF-16), a mode flag `+0x84`, a wide
line buffer and a line string (`+0xA0`); vtable `0x00AD1E58`. Opening, `ReadLine` and its UTF-16 quirks are
sc-resources.md §11 (including the wide append `0x00930818` / `0x00918AB7`); `WriteLine` (`0x009306DE`) is
graphics-rules.md §6 (the data, then CR LF, or `0D 00 0A 00` in UTF-16 mode). The remaining functions: the default
constructor (`0x0093072D`: no path, modes 0, empty buffers; the path constructor `0x009307B5` is the same after
copying the path) and the deleting destructor (`0x009307FC`).

## 5. The C++ file stream buffer (`fn:GetTrueName` owner) *[compat: library behaviour]*

Despite the owner name, these 13 functions are the linked C++ standard library's file stream buffer and output file
stream (STLport `basic_filebuf<char>` / `basic_ofstream<char>`), used by `cRZOFStream` (output-streams.md §4).
Their behaviour is the standard library's; a replacement uses its own `std::ofstream` *[choice]*:

| Address | Role |
|---|---|
| `0x0092B42C` | stream-buffer constructor (locale, empty get/put areas, vtable `0x00AE4CE4`) |
| `0x0092B4B5` | `open(name, mode)`: this on success, else null |
| `0x0092BA80` | `close()`: flushes pending output (`overflow(EOF)`) and the conversion state when open for output, closes the file, resets the areas; this on success, else null |
| `0x0092B58F` | writes the conversion's unshift sequence |
| `0x0092BB39` | sets up the code-conversion facet (width, `always_noconv`, encoding) |
| `0x0092B506`, `0x0092B52F` | release a mapped view; free the internal buffers |
| `0x0092B4EE` | `rdbuf(new)`: install a buffer, clear the state, return the old one |
| `0x0092B4D2`, `0x0092B60B`, `0x0092BBA4`, `0x0092BBC3` | destructors and deleting destructors of the buffer and the stream (virtual-base thunks) |
| `0x0092B55B` | the 64-bit byte reversal used by `cRZIStream` (output-streams.md §5) |

## 6. Function inventory

`cGZRegistry`: `0x00921BB0`, `0x00921C12`, `0x00921C25`, `0x00921C60`, `0x00921C95`, `0x00921D27`, `0x00921D49`,
`0x00921D6E`, `0x00921D97`, `0x00921DD8`, `0x00921EB2`, `0x00921F63`, `0x00921F7A`, `0x00921F9B`, `0x00921FC2`,
`0x00921FCA`, `0x00921FD2`, `0x00921FF3`, `0x00922088`, `0x009220C6`, `0x009220E2`, `0x0092212C`.
`cGZRESIDtoExtensionMap`: `0x0066F8A0`, `0x0097944E`, `0x00979473`, `0x00979B8B`, `0x00979C21`, `0x00979C5A`,
`0x0097AE66`, `0x0097B2C7`, `0x0097B3E7`, `0x0097B4E5`, `0x0097B612`.
`cGZFileNameToResKeyConverterDefault`: `0x0097942D`, `0x0097A1F8`.
`cRZTextFile`: `0x00918AB7`, `0x009306DE`, `0x0093072D`, `0x009307B5`, `0x009307FC`, `0x00930818`, `0x00930833`.
`fn:GetTrueName`: `0x0092B42C`, `0x0092B4B5`, `0x0092B4D2`, `0x0092B4EE`, `0x0092B506`, `0x0092B52F`, `0x0092B55B`,
`0x0092B58F`, `0x0092B60B`, `0x0092BA80`, `0x0092BB39`, `0x0092BBA4`, `0x0092BBC3`.

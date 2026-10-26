# `cGZDBSerialRecordAsIGZFile`: a DB record seen as a file (behaviour spec)

Scope: the `cGZDBSerialRecordAsIGZFile` owner as tools/coverage.py assigns it (24 functions, 919 bytes): the GZCOM
class `0x0A32C0DA` (registered by the resource-manager director, app-directors.md §6.1) that wraps an open
`cIGZPersistDBSerialRecord` in the `cIGZFile` interface, so that code written for files (the Gimex reader, the
video player, the Hit music loader) can read and write a record of a package.

Related: framework-rz-files.md §2 (the `cIGZFile` slot order and its meanings, here implemented over a record),
persistence.md §3 (the serial record: raw `GetFieldVoid` / `SetFieldVoid`, size, position, seeks), gimex.md §2,
intro-video.md, hit-music.md §4.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the owner was read in the decompile; the vtables `0x00AD8468` and
`0x00AD83E8` and the shared one-instruction bodies they use were read in the disassembly. Written 2026-10-24
(re-recfile7).

## 1. Object *[compat]*

| Offset | Field |
|---|---|
| `+0x00` | vtable `0x00AD8468` (the class's own interface, IID `0x8A32C0AC`) |
| `+0x04` | vtable `0x00AD83E8` (`cIGZFile`, IID `0xE020364F`) |
| `+0x08` | reference count |
| `+0x0C` | the serial record (held) |

- Constructor (`0x0096EC7C`): count 0, the given record held. Destructor (`0x0096E93A`): `Close` when a record is
  held; deleting destructor `0x0096ECAD`.
- `QueryInterface` (`0x0096DC2D`, thunk `0x0096E963`): `0xE020364F` → the file interface; `0x8A32C0AC` and 1 → the
  object. `Release` (`0x0096E96B`, thunk `0x0096EB22`): a count of 1 or less runs the final release (`0x0096DC6E`:
  close when open, then delete).
- Own interface: slot 3 `SetRecord(record)` (`0x0096DC93`: AddRef the new one, release the old one; true), slot 4
  the record, slot 5 the file interface.

## 2. The `cIGZFile` interface over the record *[compat]*

Without a record every call fails as noted. Position, size and seeks are the record's (persistence.md §3).

| Slot | Address | Behaviour |
|---|---|---|
| 3 `Open(access, creation, share)` | `0x009378BC` (shared) | false: the record is opened by its owner |
| 4 `IsOpen` | `0x0096E957` | a record is held (slot 21 is the same) |
| 5 `Close` | `0x0096DCBC` | the record is closed through its DB record interface and released; true when there was one |
| 6 `Position` | `0x0096DCE4` | the record's position, −1 without |
| 7 `Length` | `0x0096DCF9` | the record's size |
| 8 `SetLength(n)` | `0x0096DD0C` | forwards to the record's slot 53, which is a no-op returning false |
| 9 `SeekToBegin` | `0x0096DD22` | absolute seek to 0: 0, or −1 |
| 10 `SeekToEnd` | `0x0096DD40` | absolute seek to the size: the size, or −1; **an empty record gives −1** *[quirk]* |
| 11 `SeekRelative(n)` | `0x0096DD75` | relative seek, then the new position; −1 on failure |
| 12 `SeekAbsolute(p)` | `0x0096DDA5` | absolute seek: p, or −1 |
| 13 `Seek(offset, mode)` | `0x0096DDCA` | mode 0 → slot 12, 1 → slot 11, 2 → absolute seek to size + offset (−1 when the size is 0); other → −1 |
| 14 `Read(buf, n)` | `0x0096DE22` | the record's raw read of n bytes |
| 15 `ReadWithCount(buf, &n)` | `0x0096DE35` | n is first cut to the bytes left (size − position), then the raw read |
| 16 `Write(buf, n)` | `0x0096DE7E` | the record's raw write |
| 17 `WriteWithCount(buf, &n)` | `0x0096DE94` | raw write; on failure n := 0; returns true whenever a record is held *[quirk]* |
| 18 `Flush` | `0x00949ADE` (shared) | true |
| 19, 20 | `0x0094A5EA`, `0x0093878E` (shared) | `Remove`, `Rename`: false |
| 22, 23 | `0x0093877E` (shared) | `CopyFrom`: false |
| 24 | `0x00970C22` | the path: a static empty string (made on first use) |
| 25, 27, 28 | `0x00949B73` (shared) | `GetPath`, `SetPath`: nothing |
| 26 | `0x00949B47` (shared) | the path as a C string: null |
| 29 `GetOpenModes(&access, &creation, &share)` | `0x0096DEC1` | access := 0, creation := 3, share := 1; with a record, access gains 1 (read) and 2 (write) from the record's access flags (bits 1 and 2); true with a record, else false |
| 30, 31 | `0x0096E960`, `0x0092B33A` (shared) | buffer sizes: nothing; true |

So the Gimex reader's mode test (gimex.md §2) sees "readable" exactly when the record was opened for reading.

## 3. Function inventory

`0x0096DC2D`, `0x0096DC6E`, `0x0096DC93`, `0x0096DCBC`, `0x0096DCE4`, `0x0096DCF9`, `0x0096DD0C`, `0x0096DD22`,
`0x0096DD40`, `0x0096DD75`, `0x0096DDA5`, `0x0096DDCA`, `0x0096DE22`, `0x0096DE35`, `0x0096DE7E`, `0x0096DE94`,
`0x0096DEC1`, `0x0096E93A`, `0x0096E963`, `0x0096E96B`, `0x0096EB22`, `0x0096EC7C`, `0x0096ECAD`, `0x00970C22`
(`0x0096E957` is shared with other owners).

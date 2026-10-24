# Auto streams and auto records: the versioned record helpers (behaviour spec)

Scope: the owners `cSC4AutoLoadStream` (5 functions, 434 bytes), `cSC4AutoSaveStream` (4, 425),
`cSC4AutoLoadRecord` (5, 458) and `cSC4AutoSaveRecord` (5, 483) as tools/coverage.py assigns them. These scoped
helpers open one record of a save segment, read or write its leading **u16 version**, give the caller the open
stream or record, and close it when they are destroyed (foundations.md §2.10: "records opened through the game's
auto stream helpers … start with a `uint16` version").

Related: flora.md §10.3 (the stream helpers' exact sequence, their open test and the out-pointer ownership rules
AQ10/AQ12, verified on flora's records), foundations.md §2 and §3 (segment streams, the save file),
data-formats.md (segment slots 21 `OpenRecord`, 24 `CloseRecord`), persistence.md §3 (the serial record fields).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the four owners was read in the decompile; the shared close helper
`0x005D9430` was read in the disassembly. Written 2026-10-24 (re-autorec7).

## 1. Auto streams (SC4 segment streams) *[compat]*

Object: vtable (`0x00AA2B0C` load, `0x00AA2B10` save), `+4` the segment (held, `cISC4DBSegment`, IID
`0x499899DA`), `+8` the stream (held), `+0x0C` the version (load: initially `0xFFFF`; save: −1, then the version
written).

- **Constructors** (`0x005D9E70` load `(segment, key…)`, `0x005D9F00` save `(segment, key…, version)`): a null
  segment leaves the helper closed; otherwise the segment is asked for `0x499899DA` and, when it answers, the
  helper opens the record with it; the interface reference from that query is released.
- **`openStream`** (`0x005D9C90` load, `0x005D9DB0` save): the old stream is released; `OpenIStream(key, &stream)`
  (segment slot 5) or `OpenOStream(key, &stream, true)` (slot 8); then the version is read (`GetUint16`, kept at
  `+0x0C`) or written (`SetUint16`). If the stream then reports an error (slot 18) it is closed (`CloseIStream`
  slot 6 / `CloseOStream` slot 9) and released; otherwise the segment is held. Load returns true only when open;
  save returns `IsOpen`.
- **`IsOpen`** (`0x005D9670`): both the segment and the stream are held.
- **Destructor** (`0x005D9C30` load, `0x005D9D50` save; deleting destructors `0x005D9EE0`, `0x005D9F80`): when
  open, close the stream through the segment; release both.

The version read is not checked by the helper; callers compare it themselves. Details and the plugin edge cases
are in flora.md §10.3.

## 2. Auto records (GZ persist records) *[compat]*

Object: vtable (`0x00AA2AF4` load, `0x00AA2B00` save), `+4` the persist DB segment (held), `+8` the serial record
(held, IID `0xC56B8F08`), `+0x0C` the version (u16, initially `0xFFFF`).

- **`openRecord(segment, type, group, instance[, version])`** (`0x005D94C0` load, `0x005D95B0` save): a null
  segment → false. The segment is held and the record opened with `OpenRecord(key, &record, access)` (segment slot
  21; access 1 to read, 3 to write); on success the record is asked for the serial record interface. Load then
  reads the u16 version (`GetFieldUint16`, persistence.md §3.2) into `+0x0C`; save first stores the given version
  and writes it (`SetFieldUint16`). If that field call fails, the serial record is released, the record closed
  (segment slot 24 `CloseRecord`) and the segment released (the helper is then closed). The reference from
  `OpenRecord` is released in every case after the open succeeded. The result is the field call's result (false
  when the open or the interface query failed).
- **Constructors**: `(segment, type, group, instance)` (`0x005D9A20` load) and `(segment, type, group, instance,
  version)` (`0x005D9B20` save) call `openRecord` directly. `(segmentStream, key…)` (`0x005D9AA0` load, from a
  `cISC4DBSegmentIStream`, IID `0x4999A8F5`; `0x005D9BA0` save, from a `cISC4DBSegmentOStream`, IID `0x6999A8FB`)
  ask the stream for that interface, release the answer at once, take its segment (slot 25, then that object's
  slot 3, the persist DB segment) and open the record there; a stream that does not answer opens nothing.
- **Destructor** (`0x005D9490` load, `0x005D9580` save; deleting destructors `0x005D9A60`, `0x005D9B60`): the shared
  close (`0x005D9430`): with both held, the serial record's DB record interface (its slot 3) is closed through the
  segment (slot 24) and released when the close succeeds; then both references are released.

## 3. Function inventory

`cSC4AutoLoadStream`: `0x005D9670`, `0x005D9C30`, `0x005D9C90`, `0x005D9E70`, `0x005D9EE0`.
`cSC4AutoSaveStream`: `0x005D9D50`, `0x005D9DB0`, `0x005D9F00`, `0x005D9F80`.
`cSC4AutoLoadRecord`: `0x005D9490`, `0x005D94C0`, `0x005D9A20`, `0x005D9A60`, `0x005D9AA0`.
`cSC4AutoSaveRecord`: `0x005D9580`, `0x005D95B0`, `0x005D9B20`, `0x005D9B60`, `0x005D9BA0`.

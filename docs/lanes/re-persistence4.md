| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-persistence4: spec of the GZ persist runtime (records, serial record fields, persist resource/factory base, key list, buffer resources) | `re-persistence4` / (RE spec lane) | ready: new docs/specs/persistence.md (7 owners, 153 fns, 14,792 B of the 74,624-B subsystem); questions PR1-PR3; no ABI, no game runs | 1. coordinator adds the coverage-map rows below; 2. next RE lane: cGZPersistBufferResource(+Factory) TGA/BMP, then nSCRes* |

## Report

- New spec `docs/specs/persistence.md`: §2 DB record (refcount, key, access flags, Close), §3 serial record
  (QI table, native-order flag, integer fields with in-place swap on read / scratch swap on write, floats never
  swapped [quirk], strings = foundations §2.3 reader, res keys, variants = foundations §9.2 without the marker,
  table `0x00AD8C50`), §4 persist resource base (interlocked refcount) and factory base (hook slots 8-11, lock),
  §5 key list (deque, duplicates, first-match erase, unchecked GetKey), §6 cGZBufferResource (colour type -> bpp
  table `0x00449500`; load forms BMP / v0 `0x62B9DA24` / v1 / v2 QFS; v1 padded-stride copy swap quirk verified at
  `0x0097D2E7`; save v2 and 24-bit BMP `0x55D4FB53`), §7 factory (target default (3,16), write-version stack [2]),
  §9 nine conformance tests.
- Owner bytes measured with tools/coverage.py --owners (2026-10-21): DBRecord 207, DBSerialRecord 6,172 (incl.
  folded icf helpers), PersistResource 115, PersistResourceFactory 680, ResourceKeyList 1,071, BufferResource
  5,030, BufferResourceFactory 1,517 = 14,792 B.
- Proposed tools/coverage-map.tsv rows (insert BEFORE the generic `cGZPersist*` row; tab-separated
  pattern/subsystem/area/spec/core/shadow/replace): `cGZPersistDBRecord`, `cGZPersistDBSerialRecord`,
  `cGZPersistResource`, `cGZPersistResourceFactory`, `cGZPersistResourceKeyList`, `cGZBufferResource`,
  `cGZBufferResourceFactory`, each `persistence and resources	framework	persistence.md	-	-	-`. Exact names only:
  do not use a `cGZPersist*` or `cGZBufferResource*` glob (would claim the unspecified BufferResource/Binary/ASCII/
  Surveyor owners).
- Not specified (remaining ~59.8 kB): cGZPersistBufferResource(+Factory) 8,238; Binary reader/writer, ASCII and
  Surveyor writers ~9.2 kB; cGZPropertySet*/cPropertySet* ~5.3 kB; cSL*/SL ~15.6 kB; nSCRes* ~21 kB.
- Questions: PR1 (packed-file record native-order flag), PR2 (shipped 0x62B9DA24 entries), PR3 (v1 stride choice)
  in docs/specs/questions/persistence.md.
- Evidence: re_query.py export and decomp/ only; REA not used.

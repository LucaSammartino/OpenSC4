| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-persistence4: spec of the GZ persist runtime (serial records, persist resources, buffer resources) | `re-persistence4` / (RE spec lane) | in progress | see Handoff |

## Handoff

- Goal: new spec docs/specs/persistence.md for the largest coherent uncovered family of the 74,624-byte
  "persistence and resources" subsystem: the cGZPersist* runtime (DB serial record, DB record, persist resource +
  factory, key list, persist buffer resource + factory, cGZBufferResource + factory). Not DBPF (data-formats.md),
  not SC4 segment streams (foundations §2).
- Owner bytes (tools/coverage.py --owners, 2026-10-21): nSCRes* ~21.0k, cGZPersist* ~25.6k, cSL*/SL ~15.6k,
  cGZBufferResource* 6.5k, cGZPropertySet*/cPropertySet* ~5.3k.
- Done: -
- Next: read decomp/cGZPersistDBSerialRecord.c, write persistence.md §serial record.

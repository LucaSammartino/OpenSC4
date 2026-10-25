| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-persistgraph7: GZ object-graph persistence, the persist-buffer factory and property sets, persist-graph.md, persist-buffer-factory.md, property-sets.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-gzcatchall7) | **ready** (docs only). The binary graph format, the surveyor, the binary reader and writer; the persist-buffer record forms, save, resource read, file and bitmap-resource loads; the property set, its binary and XML serializers and the Hit XML reader; owners claimed: specified +12,886 bytes (+139 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 139 functions were read in the decompile. Read in the disassembly: the reader's `ReadRoot`, the writer's
`WriteRoot`, the vtables `0x00ADA340`, `0x00ADA210`, `0x00AD9FF0`, `0x00AD27D4`, `0x00AD442C`, `0x00AD43FC`,
`0x00AD22C4` and the serial record's `0x00AD9178`, the resource-manager director's factories (class IDs of the
reader, writer, ASCII writer and surveyor), the factory's `LoadFromSerialRecord`, `SaveToSerialRecord`, `SerialRead`
and `LoadFromFile`, the compressor's result rule, the XML serializer's constructor and handlers, and the string
literals (`key`, `type`, `cGZPropertySetUint32`, the number formats). A byte search of the executable found the
binary reader's, binary writer's and ASCII writer's class IDs only in the director's table. Written on 2026-10-24.

### What changed

- New docs/specs/persist-graph.md, persist-buffer-factory.md, property-sets.md.
- docs/specs/persistence.md §1: the "still outside" sentence links the three specs.

### Quirks found

- Failed scalar reads of the binary reader return the reader's address.
- The persist-buffer loader returns true for an unknown version, and dereferences a null buffer after a failed
  header read; its v1 padded copy swaps the strides like `cGZBufferResource`'s.
- The property-set map assignment copies values byte for byte; the binary property-set reader skips unknown types
  without reading them; the XML reader processes every end element, the root's included.

### Coverage

- tools/coverage-map.tsv: `cGZPersistBinaryReader`, `cGZPersistBinaryReader::cResourceInfo`,
  `cGZPersistBinaryWriter`, `cGZPersistSurveyorWriter` → persist-graph.md; `cGZPersistBufferResourceFactory` →
  persist-buffer-factory.md (all before `cGZPersist*`); `cGZPropertySet*`, `cPropertySet*` → property-sets.md.
- Specified bytes 4,857,060 → 4,869,946 (+12,886, 87.2%, `cov_local.py`).

### Not covered

- The "Any" value classes themselves and the XML parser (class `0x6B6B01D1`).
- `ConvertBufferFromFormatToFormat` (`0x00931CF0`), its own owner.

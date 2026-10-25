| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-recfile7: the record-as-file wrapper `cGZDBSerialRecordAsIGZFile`, record-as-file.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-gimex7) | **ready** (docs only). The object, its own interface and every `cIGZFile` slot over a serial record; owner claimed: specified +919 bytes (+24 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 24 functions were read in the decompile; the two vtables (`0x00AD8468`, `0x00AD83E8`) and the shared bodies
they point at were read in the disassembly. Slot names follow framework-rz-files.md §2.2. Written on 2026-10-24.

### What changed

- New docs/specs/record-as-file.md.

### Quirks found

- `SeekToEnd` (and `Seek` from the end) of an empty record returns −1.
- `WriteWithCount` returns true even when the write failed (the count is zeroed).

### Coverage

- tools/coverage-map.tsv: `cGZDBSerialRecordAsIGZFile` → record-as-file.md.
- Specified bytes 4,896,695 → 4,897,614 (+919, 87.7%, `cov_local.py`).

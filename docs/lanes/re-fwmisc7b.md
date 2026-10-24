| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-fwmisc7b: small framework owners 2 (service accessors, allocators, standard and pooled messages, reference-counting bases, locks and threads, log back ends, cursors and keys, resource director, app bases), framework-misc2.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-fwmisc7) | **ready** (docs only). 31 owners claimed: specified +9,424 bytes (+219 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 219 functions were read in the decompile. Read in the disassembly: the standard message's vtable `0x00ACEC00` and
its referenced-object setters, the fast allocator's allocate / free / reallocate bodies, the critical section's lock
bodies and the `TryEnterCriticalSection` lookup, the two log back-end vtables and the timestamp template, the pooled
message vtables `0x00ADA82C`–`0x00ADA8C4`, the cursor factory tables and the `cGZApp` tables. Service IDs come from the
pushes of the `cRZSysServPtr` constructors. Written on 2026-10-24.

### What changed

- New docs/specs/framework-misc2.md.

### Quirks found

- A successful `cRZCriticalSection::TryLock` enters the section twice for one nesting count.
- `cRZStackAllocator::New` returns untracked heap blocks after its first call.
- `cRZThread`'s constructor AddRefs its platform data without testing the allocation.
- The `RZ::` accessors cache borrowed service pointers that are never cleared.
- audio.md §3.2's "void1 / target" field of the standard message is the IGZUnknown reference (slot 33).

### Coverage

- tools/coverage-map.tsv: 29 owner rows before the `cGZ*` / `cRZ*` catch-alls (`cRZMessage2Standard` with
  foundations.md, `cRZSystemService` with framework-gz-services.md, the two log back ends with gz-framework-core.md,
  `cGZDBRecordBitmapRes` with persistence.md, `cGZApp` with application.md, `cRZThread` with foundations.md); the
  `RZ` and `ThreadPlatformData` rows of the attributed-owner block now name framework-misc2.md.
- Specified bytes 4,902,522 → 4,911,946 (+9,424, 88.0%, `cov_local.py`).

### Not covered

- The graphic system's slot 13 (`0x0098E790`, owner `cRZSystemService`) is described by its observable sequence
  only; naming it belongs to gz-graphics.md.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-property-setter-helpers4 (eleven portable property setter helpers, `property-setter-helpers.md` §§2–5) | `Codex`+`Claude` / `core-property-setter-helpers4` (engine lane) | ready: `fc4_core::property_setter_helpers` with shared acquisition and eleven typed paths; gate cargo 5732/0, clippy clean, ctest 68/68; ABI unchanged; no game runs; no questions filed | Coordinator: land via merge train. |

## Report
- `engine/core/src/property_setter_helpers/` implements the spec's normal-call contract over a
  `Primitives` trait of individual foreign calls with opaque 32-bit addresses: holder slot 9 lookup;
  on a zero result the pool free-head pop (allocator fallback when the head is zero), the live-count
  increment, block table/constructor/ID stores in spec order, and holder slot 15 install with its
  return ignored; then property slot 5 GetVariant and the typed Variant call.
- Eleven entry points: bool, u8, s32, u32, s64 (low/high), f32 (raw bits), string, and u8/u32/f32/s64
  vectors (begin then end read after acquisition, count = (end − begin) arithmetic-shifted by the element
  size, primitive called once even when empty). The result is the final call's raw `Residue` (EAX bits plus
  a defined-bit mask), not a success flag.
- `Refusal` is the portable boundary only: primitive/memory errors propagate with earlier stores kept
  (no rollback), address overflow, and vector descriptors outside the spec's valid domain (negative,
  misaligned, or over-`i32` extents, null begin with a nonempty extent).
- Tests (6): spec S1–S12 scalar traces, the twelve array literals as 24 existing/new traces, the eight
  empty-descriptor distinctions, every overload through existing/inherited/missing/allocator paths, the
  install residue/retention matrix with wrapping count, and prefix-preserving refusal.
- Gate: cargo 5732 passed / 0 failed (summed over all `test result:` lines), clippy -D warnings clean,
  CMake build and ctest 68/68 passed; rustfmt clean on the module files.
- Not done: Variant storage/conversion, pool allocator internals, x87 exceptional float behaviour,
  aliasing/reentry/exception domains (all foreign per spec §5). Not wired into engine/capi or the plugin.

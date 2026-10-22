# dates: answered questions

<!-- from: core-dates4 (2026-10-21) -->
- ~~**DT2 - §1.5:** `Set(monthName, d, y)` compares "the first three characters, ignoring case". What happens for a
  name shorter than three characters (`"Ma"`, `""`) and for a null `monthName` pointer, and is case folded as
  ASCII only or by the C runtime's current locale? The engine (`engine/core/src/dates/mod.rs` `month_from_name`)
  reads it as a C `strnicmp(name, "Jan", 3)` in the C locale: a shorter name matches nothing (January), folding
  is ASCII only; a null pointer is left to the plugin glue (the core takes a byte slice).~~
  **ANSWERED 2026-10-22 (re-dates-contracts4):** DIFFERS — §1.5 specifies the actual three-byte
  `_strnicmp(tableEntry, name, 3)` call (`0x00913148` → `0x00A044F0`). Short/empty names match nothing
  and select January; null is dereferenced, with no fallback. The executable CRT's current `LC_CTYPE`
  selects the ASCII helper or locale-aware unsigned-byte `__tolower_mt` path; no fixed C locale is forced.
  §8.1 gives independent byte/NUL/case/locale discriminators. Evidence is static native instruction/data
  tracing, not engine comparison or a game execution.

<!-- from: core-dates4 (2026-10-21) -->
- ~~**DT3 - §4 slots 9 and 41:** what do `Clone(cIGZDate**)` and `DateString(cIGZString&, format)` return in EAX/AL?
  The SDK declares `bool`; the table gives no return. The engine answers true for both (`engine/core/src/dates/com.rs`).
  Does `Clone` also write `*out` when `out` is null, and does `DateString` replace or append to the string?~~
  **ANSWERED 2026-10-22 (re-dates-contracts4):** DIFFERS — §§3.2, 4.1–4.2 specify both machine signatures.
  Clone pops one pointer argument, overwrites `*out` with a fresh object + 8, and returns full EAX = 1
  from AddRef (new count 1, independent source/clone lifetime). Null `out` faults on the store; a null
  allocation stores null to valid `out` then faults on AddRef dispatch. DateString pops the output-string
  reference address and format; calls output slot 14 Copy (native cRZString replacement, owning copy);
  ignores its return and frees the temporary. Original cleanup returns full EAX = allocator lock nesting
  remaining after unlock, normally 0 even on success; AL is its low byte. There is no checked failure,
  output rollback, or normalized success bool. §8.1 gives sentinel/reference/string/failure vectors.
  Evidence is static native instruction/data tracing; no runtime or replacement verdict is claimed.

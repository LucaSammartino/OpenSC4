| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-dates-contracts4 (DT2/DT3 native date contracts) | `re-dates-contracts4` / detached lane worktree (RE docs) | ready; substantive answers `085beee0`; DT2/DT3 answered; questions check and diff check pass; gate/game n/a; no C ABI export change | Coordinator: review/apply 2 ignored native ABI proposals and reconcile existing glue with dates.md §§1.5, 4.1–4.2. |

## Report

Substantive spec/question answers committed in `085beee0`. Scope is the existing DT2/DT3 blockers only.
No engine/tests/implementation files were read or edited; no tools, maps, runtime/default settings, public ABI
tables, game files or external files were changed. Work remained on the supervisor's detached HEAD.

### Rules established

- **DT2, dates.md §1.5:** `Set(monthName,d,y)` calls native `_strnicmp(tableEntry,name,3)`, scanning all twelve
  English abbreviations. Short/empty/embedded-early-NUL names match nothing and select January; null is
  dereferenced before changing the date. Three matching bytes suffice without a fourth readable byte.
  The executable CRT's current `LC_CTYPE` selects its ASCII helper or unsigned-byte `__tolower_mt` path;
  the setter does not force a C locale. Addresses: setter `00913131`, comparison `00A044F0`, ASCII helper
  `00A05320`, locale fold `009F047B`, month-pointer table `00B168FC`.
- **DT3 Clone, §§3.2/4.1:** one stack output-cell pointer, interface in ECX, ret pops 4. Fresh 16-byte object,
  output = allocation + 8, reference 0 then AddRef to 1; full EAX = 1, AL = 1. Source and clone own independent
  lifetimes. Existing output pointee is not released. Null output faults on store; a null allocator result
  first overwrites valid output with null and then faults on virtual AddRef dispatch. No false-return path.
- **DT3 DateString, §§3.2/4.2:** output string reference address plus format, ret pops 8. Formats a temporary,
  calls output slot 14 Copy, and frees the temporary. Native Copy performs owning replacement assignment.
  Copy's result is discarded; the original free path returns the allocator lock's remaining nesting depth
  in full EAX (AL = low byte), **normally 0 on success**. Lock vtable `00AD1EF4` slots 1/2 are `00930B35` /
  `00930B6D`; the latter explicitly restores that count after leaving the critical section. No checked
  output-null, allocation failure, Copy failure, rollback, or normalized success bool is supplied.

### Evidence and validation

Queried the read-only export first and verified instruction bytes with `re_query.py asm --bytes`, including
the comparison and locale-fold helpers, constructors, AddRef/Release, Copy/assignment, string free and
lock/unlock. Caller queries found no direct callers of the two COM wrappers; their identities were checked
against original vtable data. Both existing game call sites use the base formatter and ignore its result;
they do not establish the COM wrapper's return convention. No REA query or fallback was needed.

Independently recorded disputed byte/NUL/case/locale/output-sentinel/reference/string/failure discriminators
in ignored `build-runs/dates-contract-vectors.md` before publishing answers. The resulting **14 vector rows**
are in dates.md §8.1. They are **static instruction-trace outcomes, not executed native/game vectors**.
The locale helper probe is conditional on the specified locale mapping; no session-default locale is claimed.

Ignored `build-runs/read-date-data.py` directly reads the main checkout's PE without executing it.
Its output, `build-runs/dates-native-data.txt`, records the source EXE SHA256, all 12 abbreviation strings,
three relevant date/string vtable cells, the lock vtable, and 8 native byte spans (138 bytes) that agree with
the export's key dispatch/return paths. No original bytes or decompilation are committed.

`python tools/questions.py index` regenerated QUESTIONS.md; `python tools/questions.py check` passes.
`git diff --check` passes. Cargo/clippy/ctest and game/shadow runs are n/a for this docs-only lane.
No owner coverage, replacement, shadow, runtime execution or default-activation credit is claimed.

### Questions and ABI proposals

Moved DT2/DT3, preserving their provenance and original text, to
`docs/specs/questions/answered/dates.md`, with **ANSWERED 2026-10-22 (re-dates-contracts4): DIFFERS** rules.
Only the pre-existing DT1 format-code/language-ID question remains open; no new questions were filed.

Two proposed native-interface rows, in interfaces.tsv's exact column format, are in ignored
`build-runs/abi-rows-re-dates-contracts4.tsv` (slots 9 and 41). These are review proposals, **not applied**
ABI-table changes. C ABI exports/version are unchanged (`abi=false`).

Coordinator next: review those proposals and use the published contracts when reconciling existing native
date glue. Runtime validation remains separate; this lane makes no runtime verdict. Do not infer that
selecting a locale in a different DLL's CRT changes the executable CRT's comparison behavior.

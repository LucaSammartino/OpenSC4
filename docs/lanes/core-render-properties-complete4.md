| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-render-properties-complete4: render-properties owner contracts | Codex / `core-render-properties-complete4` (engine lane, detached HEAD) | ready: RP3-RP5 core contracts; all77 audited; 22 owner tests; cargo 4250/4250, clippy clean, ctest 67/67; smoke clean, ref_warnings=0; no coverage promotion | Bind live foreign CRT service and add native adapter/shadow evidence before owner coverage credit; RP6 remains open. |

## Changes and whole-owner checklist

Completed the portable core contracts from render-properties spec sections 1-3 and 7,
including the newly answered RP3-RP5. No native adapter, ABI table, export, generated ABI
entry, coverage row or default runtime switch changed. The core models interface identity
symbolically as the spec permits; it does not expose native vtables or the 0x30-byte layout.

- Construction and registration: independently owned tables and names, all 38 Boolean,
  24 integer and 15 float entries; defaults, stored indices and extra fields preserved.
  The entire inherited registry was audited against section 3, not only used entries.
- Construction helpers: remove the initial registration byte, select uppercase bytes
  using supplied `isupper`, lower selected acronym bytes and full names using supplied
  `tolower`. Byte names support non-UTF8 output; UTF-8 convenience views return None
  when conversion is impossible. RP1/RP2 metadata stays uninterpreted.
- Slots 0-2: both P and U views share the count, initialized to zero. QI accepts only
  integer IID 1, writes U, and takes one reference. Failure preserves output and count;
  unsupported queries may omit the output cell. AddRef wraps modulo 2^32. Release
  decrements only above one; both zero and one destroy/free and return zero.
- Ordinary/deleting destruction: drop float, integer, then Boolean tables. Both
  destructor views return symbolic P; only flag bit zero frees the actual outer Box.
  Flags zero/two retain that allocation with contents destroyed until wrapper cleanup.
  The observation record is not a usable returned reference. Core post-termination
  rejection is a safety extension, not supported native stale-pointer behavior.
- Slots 3-5: counts use table length, never reserved capacity.
- Slots 6-8 and 15-17: value and byte-name access for valid indices, including zero and
  37/23/14. Byte-name getters are complete; string getters are UTF-8 conveniences.
- Slots 9-11: supported Boolean values, signed 32-bit integers and binary32 floats;
  setters preserve metadata and ignore registered ranges. Independent endpoint,
  instance-isolation and range-discriminator tests pass.
- Slots 12-14: take byte inputs, scan the first NUL, own the query copy, sign-extend
  each byte into the primitive lowering callback, store only the callback's low byte,
  and compare equal-length byte strings. Full name precedes acronym within each
  entry; first table position wins, independent of the entry's stored index field.
  No trimming, Unicode decoding, query-prefix removal or original-owner Find results.
- Invalid indexing: None/false without mutation is explicitly core safety only.
  Negative, count, signed-minimum and signed-maximum indices are tested across all
  types. No native invalid-index probes or expected native failure value are claimed.
- Threading: immutable getters use no RefCell; owner/data Send+Sync and concurrent
  immutable access are tested. Refcount and property mutation require exclusive
  Rust access; the native adapter will need its own synchronization boundary.

## Locale service and residuals

`ByteCtype` accepts only primitive signed-byte CRT `tolower`/`isupper` operations.
The core owns all name/query algorithms. Construction receives its construction-time
service and each explicit lookup receives its call-time service; changing services
does not rebuild stored names. A live foreign CRT implementation can follow the game's
thread/global locale rather than supply original-owner Find results.

`CLocale` and convenience methods deliberately select the spec's C-locale profile.
They do not resolve RP6 or follow the game's active locale automatically. Synthetic
services prove sign extension, return-byte truncation, non-UTF8 names and changing
construction/lookup profiles; they establish no shipped language/codepage behavior.
RP6 remains open in `docs/specs/questions/render-properties.md`; no new questions filed.

Next: bind a primitive service to the active game CRT (or supply proven foreign tables),
implement actual P/P+4 native interface adjustments, and add native/shadow evidence.
There is no owner adapter/shadow in this patch. No whole-owner implementation, verified,
replacement or default-runtime coverage promotion is requested from constructed tests
or from the smoke run. The core algorithms are ready for integration with that boundary.

## Measured validation

- Final `qbuild all` on unchanged source at `725ae9cd`: PASS, cargo 4250/4250,
  clippy exit 0 / zero diagnostics, native build 2/2 edges, full ctest 67/67
  (149.2 seconds). Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-build.log`, `fc4verify-ctest.log`.
- Quick gate: core 2383/2383, dependent crates compile-only, native build 4/4.
  All 22 render-properties tests pass: ten inherited tests updated to the contracts,
  seven new lifetime tests and five new independent byte/endpoint/profile tests.
- Scoped `qbuild fmt`: PASS on all four touched Rust files; no mass reformat.
- Independent registry audit: all77 names, acronyms, defaults and extra fields match
  the exact spec registry section; `build-runs/audit-render-registry.py` records the
  check. Existing independent vectors also cover all77 runtime entries, and new
  tests cover their full names/acronyms followed by NUL and non-UTF8 suffixes.
- Game label `rp-complete4-r1-core-smoke-20261022a`: clean, ref_warnings=0,
  foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0; turbo end-date request
  2026-04-01, observed exit date 2026-04-10. DLL SHA256 prefix `cbb2b337`, built from
  the runtime source at `67561d7f`; subsequent code changes only remove test borrows.
  Verdict/log are under `build-runs/game-results/`; zero render-properties shadow
  lines. This is DLL load/city/quit smoke evidence only, not owner exactness evidence.
- Initial baseline all: old cargo 4238 passed, clippy clean, native build 206/206;
  editing before it finished invalidated its ctest and tree-stability verdict. It is
  not claimed as a clean baseline pass. An intermediate quick fixture failure was
  corrected (`reeu` acronym), and six intermediate clippy test-borrow errors were
  fixed before the final complete PASS. No final failures remain.

## Commits

- `2cead6e7`: substantive RP3 lifetime and byte-ctype/NUL implementation and tests.
- `67561d7f`: completed quick validation, table-position lookup and optional QI cell.
- `725ae9cd`: six test-borrow fixes and measured game/gate record.

No C ABI exports changed. No game requests outstanding. Detached HEAD is intentional;
the supervisor publishes the actual committed tip to the reserved lane branch.


### Coordinator core-stage review (train80)

The portable owner algorithms are complete: lifecycle/QI/deleting flags, all77 registry helpers, byte/NUL query/name operations and table access/mutation are owned and independently tested. ByteCtype exposes primitive FOREIGN CRT classification/lowering only; it does not supply original render-properties results. After the full train gate/regress, core-stage credit is appropriate for cSC4RenderProperties* even though the worker conservatively requested native binding first. RP6 active-game CRT binding, actual native interface pointer adjustment, shadow and replacement remain separate unresolved integration stages; this batch grants none of those. Safe stale-pointer/index rejection is outside the native valid-input contract.

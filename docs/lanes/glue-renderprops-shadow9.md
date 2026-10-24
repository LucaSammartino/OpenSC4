| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-renderprops-shadow9: render properties shadow | Codex, `glue-renderprops-shadow9` | in progress | Round 2: extend owned-fixture methods, preserve 542-field registry, measure house through 2032. |

## Handoff

- Goal: merge RP7, extend independently predicted values/setters/lifetime/view coverage, measure house to 2032.
- Merge conflict resolution retains both additive registrations; own open RP7 removed in favor of answered pointer.
- RP7 supplies no continuous trace bridge or public snapshot getter. Never seed live expected values from native outputs or borrow SCRes debug slots.
- Done: merged RP7 in 88601f276; source now adds isolated native constructor/assignment/lifetime fixtures and Rust expectations.
- Fixture plan finite-v1: two owners; 462 completed setters, all 77 values after every setter, second-instance independence, P/U QI/count/ref operations, U destructor flags 0/2.
- Focused transport tests 2/2 passed; owned-file fmt passed after two local formatting fixes.
- RP8 appended for literal receiver-only constructor/outer-storage ABI; no freeing Release at 0/1 is attempted.
- Gate 1 on 5739fb9f1: Rust 6999/6999, clippy clean, native 221/221; CTest 78/80 (ret-channel falsely associated fixture Destroy with Lua Destroy; capi_smoke cached train-integration ABI97 object linked Rust98).
- Gate 1 logs preserved as build-runs/renderprops-r2-gate1-{test,clippy,build,ctest}.log; queue qbuild-20261024-094717-build-ctest terminal fail.
- Diagnostic fixes: specific fixture termination name; capi_smoke mismatch includes source provenance and forces a current-source rebuild. Header and Rust are both ABI98; no version edit.
- Next: repeat full qbuild after these diagnosed changes, then unique-label house request through 2032.
- Constructor is specified at render-properties.md §1.3: 0x007E3650, complete object 0x30 bytes.
- No new game request outstanding; previous evidence below is historical round 1.
- tools/questions.py index is excluded by non-bending clean-room tools rule; coordinator must regenerate index.

## Historical round 1 result and scope

Added default-off `renderprops_shadow = 1`, including the recognized-key list, observer load/month/shutdown hooks, and private static-link Rust expectation transport. Public C ABI exports are unchanged. Each sample compares **542 fields**: three counts, 77 full names, and 462 Find results. Six queries per entry cover full name, abbreviation (including first-match collisions), uppercase, embedded NUL, a forbidden leading `k`, and a trailing space. Expectations come exclusively from a fresh production `RenderProperties` core, using its explicit C-locale ASCII profile. Native outputs never seed or resynchronize expected state.

The original application owner remains active. The shadow acquires the application interface, keeps it live while using its borrowed properties pointer, and releases only that acquired application reference. It never writes native properties, retains native pointers between samples, or arms hardware debug registers. A month guard emits exactly one dated `renderprops shadow YYYY-MM-DD: ... mismatches=N` line per sampled month, including load.

Compared owner methods: `NumBoolProperties`, `NumIntProperties`, `NumFloatProperties`; `NameFromBoolPropertyID`, `NameFromIntPropertyID`, `NameFromFloatPropertyID`; `BoolPropertyIDFromName`, `IntPropertyIDFromName`, `FloatPropertyIDFromName`.

**Uncompared owner methods/state:** `BoolValue`, `IntValue`, `FloatValue`; `SetBoolValue`, `SetIntValue`, `SetFloatValue`; properties `QueryInterface`, `AddRef`, `Release`, construction/destruction, P/U lifetime and independent-instance copying. The separate view-owned instance is not acquired. Extra metadata lacks public getters (RP1/RP2); non-C locale/high-byte behavior remains RP6. Registered defaults cannot predict configured values after startup/preferences writes, and no supported setter-input history/observer is specified. These omissions mean **no whole-owner verified credit**.

## Measured game results

- `renderprops-shadow9-20261024-r2 73/73`: final DLL `f179d2f8`, turbo with the supplied train187 house switches plus this key, **2026-01-27 through 2032-01-01**, 39,566 field comparisons, zero duplicate months. Clean shutdown at 2032-01-01, 939.625 real seconds; foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0, other WARNING lines=0. [Run 491](../runs/renderprops-shadow9-20261024-r2-491.md).
- `renderprops-shadow9-20261024-r1 43/43`: initial DLL `0775c3ab`, same house profile, clean shutdown at 2029-07-11 after 602.094 seconds; time budget ended before target. The run exposed an unknown-key warning despite an active shadow. Key registration was corrected and the entire gate repeated before r2. [Run 490](../runs/renderprops-shadow9-20261024-r1-490.md). Run IDs were moved out of the concurrently occupied 469-471 range.

Both unique-label verdicts/logs remain under `build-runs/game-results/`; no request is outstanding. Event-based waiters were collected to terminal results; nine-minute file-wait timeouts caused only waiter restarts, never duplicate game requests.

## Checks

Final engine source is `e1937e01f`; subsequent commits change documentation only. Final `qbuild.py all` PASS: workspace cargo **6985 passed, 0 failed, 0 ignored**; clippy `-D warnings` PASS with zero diagnostics; coordinator native build **95/95 edges** (4.6 s), full **CTest 79/79** (96.7 s). Native queue record: `qbuild-20261024-085733-build-ctest`. The all operation itself queued build/ctest, so no duplicate verify was submitted. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.

Focused Rust transport test passed 1/1 (579 filtered); owned-file formatting passed for `capi/src/renderprops_shadow.rs`. Earlier source `aa6a2508b` also passed all (6985 Rust tests, clean clippy, native 254/254 edges, CTest 79/79; queue `qbuild-20261024-083622-build-ctest`). Initial wrong-relative-path fmt and unavailable Bash-wrapper attempts are distinct from the successful checks.

## Questions and next steps

Appended **RP7** to `docs/specs/questions/render-properties.md`: specify ordered live setter inputs/observation and public view-instance acquisition. Graphics-rules sections 1/5/8 establish startup/preferences writes and persistence but no read-only history service. Implement all supported registry comparisons now; leave value/lifetime/view scope uncredited until a valid independent contract and house-safe experiment exist.

Coordinator: refresh the generated questions index and review partial registry credit. Coverage tables and `tools/regress.ps1` were not accessed or edited: the clean-room tools exclusion and detached/no-merge instructions apply. Any shared profile append belongs to the coordinator after review. The shadow stays default-off.

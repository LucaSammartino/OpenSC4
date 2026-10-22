| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-command-system4: native command verification | Codex / `verify-command-system4` (engine lane, detached HEAD) | in progress; round 2; historical 7414 assertions / 7243 exact / 171 scalar metadata differences | 1. Fix production native count projection. 2. Gate and diagnose native save control. 3. Validate clock24 and exact house40 save/reload. |

## Handoff

- Goal: implement published scalar GetCount=0 / array length in command production projection, preserving Variant::count.
- Done: production fix 82376d6b; previous-round history b2b4dd2d and below. Baseline: Rust 4961 PASS, clippy clean, ctest 68/68.
- Implemented: borrowed NativeParameter production view on stored command values/fallback; native count 0 for scalar/Empty, array length. Logical Variant::count untouched.
- Verifier uses that production view, retains all 7414 assertions and separately logs historical logical count differences. Scalar/empty/one/multi/string/fallback tests added.
- Focused: 3466 tests PASS, dependents compile; fmt all 4 touched Rust files PASS. Stable all gate on 82376d6b: Rust 4962 PASS, clippy clean, ctest 68/68.
- Immutable DLL: build-runs/commands4-r2-frozen/FreeCity4.dll SHA256 17115A0C565DD4E63D8191255C47B50CA9D08561580C0F3A445BF4CD5B169396.
- Native controls commands4-r2-native-on-82376d6b-a and commands4-r2-native-off-82376d6b-a BOTH stalled in identical 2026-03-01 full-save script, ref0; verifier-off emitted no comparisons. Native whole-profile save still FAILS.
- Native-on: 7414/7414 exact, 0 refused, historical logical count differences 171. Independent ORIGINAL objects, no native output seeding.
- Clock24 commands4-r2-clock24-82376d6b-a CLEAN through 2032-01-14/ref0, save full ok=1 in 10906 ms; same 7414 exact/171 historical.
- House40 commands4-r2-house40-82376d6b-a CLEAN through 2032-01-01/ref0; full save ok=1 in 10969 ms, PostSystemServiceShutdown reached, 7414 exact/171 historical.
- Clock24 also reached PostSystemServiceShutdown. Native whole-profile failures remain explicitly failures.
- Next: stage successful house40 save with timestamp validation (build-runs/commands4-r2-stage.py), then supported house40 reload through 2032. No request outstanding; helper build-runs/commands4-r2-run.py waits without qwait.
- Shared variant.rs/variant CAPI, tools, maps, ABI tables and defaults are excluded.
- Preserve all 171 comparisons and raw logical-count history. Native owned sets remain game-created originals.
- GC1-GC7 remain outside scope. House40 uses exactly the 40 supplied opt-ins, existing simulator/budget infrastructure defaults; no terrain/effects/dispatch.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-verify-command-system4-r2-manifest.json.

## Changes and provenance

Added the opt-in `commands_verify = 1` city-load comparison in `engine/plugin/FC4CommandsVerify.h`,
called additively from the observer. `engine/capi/src/commands_verify.rs` retains actual production
`CommandParameterSet` objects and calls production methods and `StringCommandGenerator` parsing.
It is private static-link transport: no public C ABI exports, export maps, ABI tables, defaults or ABI version changed.
Implementation committed at `39531ad2`, return-channel correction at `f844935e`, final instrumentation at `5eb61637`.

Bindings were confirmed before baseline builds, within the initial 15-call audit: command-system section 5,
SDK command/variant declarations, `GZServPtrs` and `cRZSysServPtr`. The app-owned command server is acquired
through public service `0xEB903A32` / IID `0xCB90253A`. Slot 5 makes independently owned ORIGINAL sets.
An additional ORIGINAL scratch set supplies native variants; SDK strings only carry input/output bytes.
No private native storage layout is needed. Refaudit recorded game vtables at module+`0x6D17A0` for the service
and module+`0x6D16C8` for the sets, consistent with the published class tables.

No commands are registered or executed. No global dispatcher, generator or app-owned server is replaced or shut down.
Created references use audited created-release; the acquired service uses audited acquired/release.
Core receives identical raw literal inputs, never native computed output or a resynchronized native map.

## Measured domain

Owned-set insertion, overwrite, deep string copy after source mutation, set-to-set copy, typed retrieval,
missing lookup and writable fallback, sparse unsigned order, iteration, removal, count growth/shrink/clear,
append after the highest ID, wrapping append after `0xFFFFFFFF`, and ordinary signed status storage.
Typed inputs cover empty, strings, true/false, positive/negative Sint32, high Uint32, and exact positive/negative Float32.

Fourteen generator cases cover known/unknown commands, unchanged outputs on blank input, case-insensitive name lookup,
positive/negative numeric and boolean literals retained as strings, whitespace/tabs/newlines, empty quoted words,
embedded/unbalanced quotes, explicit input length, strlen sentinel, and truncation after 256 words.
The core server fixture has the fixed spec section-0 `SetDebugLevel` name/ID; the app prerequisite is checked separately.
Native lookup results never populate that fixture. Parse-only converting/execution callbacks refuse unused operations.

Each completed fixture makes **7,414 assertions**: 3,708 set assertions and 3,706 parser/prerequisite assertions.
These include repeated observations of retained objects; they are not counts of unique methods or commands.
On the final DLL, **7,243 match, 171 differ, zero refuse**. Aggregate counters prove all 171 differences concern
variant element-count metadata; **zero other fields differ**. Native scalar `GetCount()` is 0, production
`Variant::count()` is 1. [GC8](../specs/questions/command-system.md) records the native-interface versus logical-count
binding question and cites the already-published data-formats scalar-count contract. No semantics were fitted.

GC1-GC7, secondary QI identities, ValidateParameter, any-parameter comparison, status meanings, dispatch/nesting,
non-space delimiters, VoidPtr conversion and generator formatting remain outside measured coverage.
This is bounded verification, with no whole-owner or replacement promotion claim.

## Gates

Baseline: 4,961 Rust tests PASS, clippy 0 diagnostics, ctest 68/68 PASS.
Final source `5eb61637`: focused quick 530 capi tests PASS; full qbuild PASS with 4,961 Rust tests,
clippy 0 diagnostics, C++ build PASS and ctest **68/68** (99.3 s). Return-channel audit passes without map edits.
New Rust transport fmt PASS. Checking the touched `lib.rs` recursively reports pre-existing formatting in
unchanged children (first `advisors.rs`); no mass reformat performed. Wrapper fmt requires absolute paths.

## Game evidence

All labels are unique; requests were sequential and waited through the local shell helper, without qwait.
Completed verdicts were retained. The original comparison itself takes about 2 ms at load.

- `commands4-native-r1-e50c5bbc-a`: all replacements disabled; **stalled** harness verdict. Reached 2032-01-01,
  ref_warnings=0, full save succeeded in 10860 ms and full shutdown completed. One watchdog stall during saving.
  7,414 / 7,243 exact / 171 differences / 0 refusals; only the first twelve differences were individually logged.
- `commands4-native-r1-e50c5bbc-b`: all replacements disabled; **stalled** during early full save at 2026-03-01,
  ref_warnings=0. 7,414 / 7,243 exact / 171 element-count differences / 0 other differences / 0 refusals.
- `commands4-own-r1-e50c5bbc-b`: ordinary plugin defaults (simulator and budget; **not current40**), **clean**
  through 2032-01-03, watchdog_stalls=0, ref_warnings=0. Full save succeeded in 11109 ms and full shutdown completed.
  Same 7,414 / 7,243 / 171 element-count differences / 0 other differences / 0 refusals.
- `commands4-reload-r1-e50c5bbc-b`: supported harness fixture reload of that successful save, **clean**
  through 2032-01-14, watchdog_stalls=0, ref_warnings=0. Full save succeeded in 12297 ms and full shutdown completed.
  Same 7,414 / 7,243 / 171 element-count differences / 0 other differences / 0 refusals.

The successful own-save source timestamp was validated against its run window before copying the 1,018,906-byte
save to main `build-runs/fixtures/commands4-own-e50c5bbc.sc4`; harness staging records hash prefix `ff38a466`.
No save or game file is committed. Logs/verdicts are under this worktree's `build-runs/game-results/`.
The full label-b gate and all label-b games use frozen DLL SHA256
`1F8594269BE9926AB1EA047D721195B6D5C03819B6D2A99194C0D7DB4B5A16D2`.
Label a used `B633D033A3778553DC6F6AC253466FE3FDB58BA31B4A997B4146EE9A8D285375` before aggregate instrumentation.

## Remaining work

WIP: resolve GC8's production logical-count/native-interface distinction without fitting command semantics.
Obtain the canonical current40 profile from the coordinator: the permitted named files identify it but do not
publish its exact settings, and the two default-on keys must not be substituted. Complete its guarded regression
and a clean native save/shutdown run; both all-native full-save attempts here stalled. No requests remain outstanding.
No restart, merge, rebase, push, subagent, shared-tool or ABI-table change was performed.

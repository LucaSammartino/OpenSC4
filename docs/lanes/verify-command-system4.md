| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-command-system4: native command verification | Codex / `verify-command-system4` (engine lane, detached HEAD) | ready; source 82376d6b; cargo 4962 PASS, clippy clean, ctest 68/68; 7414/7414 native assertions exact, 171 historical logical-count differences retained; clock24/house40/reload clean 2032/ref0; native verifier-on/off full-save controls both stalled | 1. Integrate bounded production projection. 2. Native whole-profile save watchdog failure remains separate baseline work; GC1-GC7 excluded. |

## Round 2 result

Production fix **82376d6b** adds a borrowed `NativeParameter` view to the command parameter API.
It projects native variant metadata from the actual stored value or shared fallback: scalar/Empty count 0,
array count equal to its length, per data-formats §5.2 and §10.5 item 42. `Variant::count()` remains the
logical-element API. The shared variant implementation and variant CAPI were not edited.

The command CAPI verifier now calls this production view. All **7,414 original assertions remain**:
3,708 set and 3,706 parser/prerequisite assertions. Every measured run with verification enabled reports
**7,414 exact, zero mismatches, zero refusals**, and separately **171 historical logical-count differences**.
The round-1 result **7,243/7,414 with 171 differences** is retained below. Those were native raw-count versus
logical DTO-count comparisons, rather than differences in command semantics. No expected input was changed,
no field was ignored, and native answers never populate or resynchronize Rust state.

The native objects remain independently owned ORIGINAL sets created by the app-owned command server;
the scratch variants are ORIGINAL too. No SDK parameter-set reimplementation replaces the object under test.
No command registration/execution or replacement was introduced. GC1-GC7 and the other previously excluded
behaviors remain outside measured scope; this is not exhaustive or whole-owner verification.

Tests cover scalar metadata, Empty, empty/one/multiple-element arrays, strings, present/missing lookup,
borrow identity, removal and writable shared fallback, while asserting preserved logical counts.
Baseline all: **4,961 Rust tests PASS, clippy clean, ctest 68/68**.
Focused: **3,466 tests PASS**, dependent crates compiled, plugin built; fmt of all four touched Rust files PASS.
Stable full gate on source 82376d6b: **4,962 Rust tests PASS, clippy zero diagnostics, ctest 68/68**.
Subsequent commits change only this report. Public C ABI exports, ABI version, maps, defaults and shared tools
are unchanged. No new spec question was necessary: the published rule resolves GC8's adapter distinction.

## Round 2 real-game results

All five requests used the same immutable `build-runs/commands4-r2-frozen/FreeCity4.dll`, SHA256
`17115A0C565DD4E63D8191255C47B50CA9D08561580C0F3A445BF4CD5B169396`, matching the gated build DLL.
Every request uses the identical `2026-03-01 save` script, end date 2032-01-01, refaudit and full shutdown request.
New unique labels were awaited without qwait; completed verdicts were retained. All runs have **ref_warnings=0**.

| Label | Verdict / final date | Full save | Full shutdown | Native command comparisons |
|---|---|---|---|---|
| commands4-r2-native-on-82376d6b-a | stalled / 2026-03-01 | interrupted by watchdog | not reached | 7414 exact; historical 171 |
| commands4-r2-native-off-82376d6b-a | stalled / 2026-03-01 | interrupted by watchdog | not reached | verifier OFF; no comparisons emitted |
| commands4-r2-clock24-82376d6b-a | clean / 2032-01-14 | ok=1, 10906 ms | PostSystemServiceShutdown | 7414 exact; historical 171 |
| commands4-r2-house40-82376d6b-a | clean / 2032-01-01 | ok=1, 10969 ms | PostSystemServiceShutdown | 7414 exact; historical 171 |
| commands4-r2-reload-82376d6b-a | clean / 2032-01-01 | ok=1, 10937 ms | PostSystemServiceShutdown | 7414 exact; historical 171 |

The two native controls disable every replacement. Their requests differ only in label and `commands_verify`.
Both hit the no-OnTick watchdog at about 5 seconds and terminate at about 10.2 seconds during the deferred
full-save action, outside FreeCity4. Turbo is explicitly ineffective with the replacement simulator disabled.
The engine watchdog code lowers the outside-FreeCity4 sampling threshold to 5 seconds during autotest
(`FC4Watchdog.cpp`); this is a measured watchdog termination, not proof of a permanent native save deadlock.
The identical verifier-OFF failure establishes that the observed stall does not require this comparison.
**The all-native whole-profile full-save/shutdown regression still fails and is not claimed passed.**
No watchdog suppression, shared harness modification or unsupported callback change was made.

Clock24 adds only `clock24` to the existing simulator/budget infrastructure defaults. House40 and reload add
exactly these supplied 40 opt-ins to the same defaults:
`landvalue,weather,ordinance,demand,power,plumbing,flammability,pollution,protection,crime,aura,flora,landfill,residential,neighbors,zones,development,tract,history,clock24,commercial,industrial,civic,parks,lotdev,zonedev,traffic,automata,netmgr,netlot,nettool,demolition,occmgr,propmgr,lotcfg,citysit,propdev,simgrids,citydetail,ports`.
No experimental terrain/effects/dispatch was enabled. The command comparisons still exercise native components
independently in these supported profiles. There are no current differences requiring a normal-speed rerun;
the retained logical-count distinction also occurs in the native controls where turbo has no effect.

The house40 save was staged only after validating its modification time **2026-10-22 19:46:30.116853** against
the successful run window and its DBPF signature. Fixture: main `build-runs/fixtures/commands4-r2-house40.sc4`,
**998,674 bytes**, SHA256 `4ad4e47b96e66110216ce0a2794c7f0cd6cdb5450295b97d320e7e94469efff8`.
The supported reload verdict confirms that same full hash was staged, with source preserved.
No game artifact is committed. Logs/verdicts and lane-only request/staging/audit helpers remain under this
worktree's `build-runs/`. No request remains outstanding. Native baseline watchdog/save behavior is separate
follow-up work; existing GC1-GC7 coverage remains unclaimed.

## Round 1 historical record (b2b4dd2d; superseded by round 2 above)

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

## Round 1 remaining work (historical; resolved as recorded above)

WIP: resolve GC8's production logical-count/native-interface distinction without fitting command semantics.
Obtain the canonical current40 profile from the coordinator: the permitted named files identify it but do not
publish its exact settings, and the two default-on keys must not be substituted. Complete its guarded regression
and a clean native save/shutdown run; both all-native full-save attempts here stalled. No requests remain outstanding.
No restart, merge, rebase, push, subagent, shared-tool or ABI-table change was performed.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-language-native4 (native utility verification) | `Codex` / `verify-language-native4` (engine lane; detached HEAD) | in progress FINAL round3/3: retained source06d71bb2, cargo4961/0, clippy0, CTest68/68; eight historical attempts preserved; LU1-LU4/SL1 open | (1) Audit immutable2/source/gate; (2) supported simulator-only OFF then matched ON; (3) conditional literal40 plus simulator/budget defaults; stop on prerequisite failure. |

## Handoff

- Goal: FINAL bounded round3 supported lifecycle controls; no source/owner/default expansion, no fourth round.
- Done: initial round3 handoff9ca61786; all round2 ledger and copied evidence reviewed; engine tree b5044770680594c46e936d71a4b5bec83a63c10f equals 06d71bb2; no working/untracked engine changes.
- Done: built DLL/PDB and immutable2 hashes match; retained cargo4961/0, clippy0, CTest68/68 audited from logs (CTest93.03s); no redundant rebuild/fmt.
- Next: submit unused label vlun4-r3-supported-off-2032-i; require clean/ref0, returned full save, completion mark and PostSystemServiceShutdown before any ON request.
- New hypothesis: disable=all/turbo0 historical failures differ from supported simulator-only/disablebudget/turbo1; prior OWNc already passed the latter effective profile.
- Requests planned: unique probeOFF first, then exact matched ON if OFF passes, then literal40 enable/defaultbudget if ON passes; seconds420/timeout600, Konradshohe, target2032-01-01.
- Immutable2 expected DLL SHA256: 78ea9b099f55d8abd95ea76cea21f34ced2caf72fc68ec305f35da43185c9342.
- Retain strict116235/116235 separately from foreign host2484/4140 and question gaps2629/3043 per enabled run; 46 constructed lifetimes, natural0/0.
- No request outstanding at this commit; OFF request will follow. Use existing copied evidence-run4/cmdline4-r3-supported-save.script unchanged, SHA25670a903f02158b86981fbc3d5411c8e2ace5a9ea6539b5ab3806f0ae07fa24ac6; no cmdline DLL use.
- No supported isolated reload fixture supplied; reload remains unmeasured. LU1-LU4/SL1 open. q.py Bash unavailable (access denied); run bounded lane-local Python directly.
- Guard: C:/Users/tlori/FreeCity4/build-runs/run4-verify-language-native4-r3-manifest.json; original family final3/3, maxCalls100.

## Round2 diagnosis and controls (2026-10-22)

Round2 of the original at-most-three lane rounds. No engine source, production algorithm, host provider, callback, lifetime policy, default, ABI table or tool was changed this round. By call30 the native no-verifier baseline had reproduced the failure; no owned language fix was justified. The late current40 warnings identify other provider shutdown sites and do not justify fitting language behavior or changing unrelated owners in this lane.

The baseline and final `qbuild all` both passed: **cargo4961/0, clippy0 diagnostics, CTest68/68**. Final CTest93.4s; `qbuild: PASS`. No Rust files were touched this round, so no new fmt or focused source test was required. The immutable2 DLL/PDB from round1 were reused unchanged; the rebuilt DLL SHA256 remains `78ea9b099f55d8abd95ea76cea21f34ced2caf72fc68ec305f35da43185c9342`. PDB SHA256 is `ab9ee7606d4ea62a51ab6bccc142bc989264334c55aab2dfbd384ef4cd406786`.

### Native full-save isolation

`vlun4-r2-native-noprobe-save-f` differs from failed corrected native request `vlun4-r1-native-06d7-2032-d` only in unique label and `language_native_verify=0`. Both use the identical immutable2 DLL, original simulator/budget (`disable=all`), normal speed, 2032-01-01 target, 600s city limit, refaudit, and identical script `2031-12-01 save`. Script SHA256 is `1b3ef9d7527e13a85eb1ec4305489c72444358b1cd20daa5029cc38aced7f24f`. Session seeds are naturally different; this is a failure control, not a deterministic simulation comparison.

The no-probe baseline **stalled2031-12-01**, with ref_warnings0 and foreign_exceptions0. No language BEGIN/END, constructed utility or host callback executed. Full save did not return; neither full shutdown hook completed. This is a sixth retained attempt, not a replacement for any prior failure.

Own-DLL/PDB symbolization of the eight distinct FreeCity4 return-address candidates in each copied stack is identical for probe-on and probe-off logs:

| DLL offset | Our source location |
|---|---|
| 0x916AD0 | FC4Diag.cpp294, SehGuard |
| 0x9178F7 | FC4Diag.cpp239, RunGuarded |
| 0x9312FF | FC4Observer.cpp904, ObserverSaveCity |
| 0x933087 | FC4Observer.cpp517, SaveStep (game interface SaveCity pending) |
| 0x9332D0 | FC4Observer.cpp572, observer OnTick |
| 0xD51ED1 | FC4Script.cpp539, ScriptOnTick |
| 0xD531C7 | FC4Script.cpp269, RunSave |
| 0xD54777 | FC4Script.cpp441, RunAction |

These are sampled stack candidates, not an unwound proof of every live frame. No executable bytes or foreign function symbols were read. The watchdog's owned source explains termination after more than10s without a tick; the last published native hidden-pause state is0, while the passing owned save's is2. A long save and watchdog/setup interaction remain possible; the game's internal save cause is not established. The same save-boundary failure occurs with the verifier disabled, so the evidence does not support changing owned language callbacks/providers/references. This does not establish native full-save compatibility.

Diagnostic artifacts remain lane-local: `build-runs/symbolize-language-owned.py`, `language-probe-on-symbols.txt`, `language-probe-off-symbols.txt`, `audit-language-round2.py`, and `language-round2-audit.txt`. Symbol candidate comparison reports no differences.

### Corrected literal current40

`vlun4-r2-current40-save-g` is the seventh retained attempt. Its enabled providers are the default owned simulator plus exactly the40 supplied names; budget is disabled, and terrain/dispatch/effects are absent. The request contains the literal list, rather than an inferred profile. It reuses immutable2 and the same full-save script, turbo1, target2032-01-01.

Result: **clean2032-01-01, ref_warnings4**, foreign_exceptions0, watchdog_stalls0; full save returned1 in11031ms; PostAppShutdown and PostSystemServiceShutdown completed. Constructed comparisons are still **strict116235/116235**, **host2484/4140** (1656 differences), **gaps2629/3043** (414 differences), objects46, refused0, resync0, fallback0, managerUnchanged1. Natural0/0 remains unexercised. Every raw difference is retained and assigned to LU1-LU4/SL1 as in round1; the foreign printf/actual-host boundary and codec coverage gaps remain open.

All four reference warnings are class0xE990BFFC at shutdown, after the constructed language probe completed: airport and seaport `DropObject:144`, parks `DropObject:126`, landfill `ReleaseToken:465`. They are retained failures of the required ref0 criterion. They are not ignored because the harness verdict says clean.

`vlun4-r2-current40-noprobe-h` is an eighth, justified attribution control: identical current40/script/artifact, verifier0. Result: **clean2032-01-01, ref_warnings4**, foreign_exceptions0, watchdog_stalls0; full save returned1 in11016ms; both full shutdown hooks completed. No language comparison/callback executed. The same four department-budget warning sites/class IDs reproduce without the verifier. This establishes their occurrence independently of language verification, not a complete diagnosis of those owners' lifetime contracts. Both literal40 enabled-provider sets were checked: exactly the supplied40 plus the default simulator, no terrain/dispatch/effects. No unchanged failed native setup was repeated. No request remains outstanding; no additional game request is justified without a changed artifact/setup and a concrete hypothesis.

### Complete retained attempt ledger

| Unique run label | Harness verdict | Ref warnings | Full save | Full shutdown |
|---|---|---|---|---|
| vlun4-r1-own-0641-2032-a | clean2032-01-01, failed ref criterion | 359 | returned1,11078ms | completed |
| vlun4-r1-native-0641-2032-b | stalled2031-12-02 | 359 | no return | not completed |
| vlun4-r1-own-06d7-2032-c | clean2032-01-08 | 0 | returned1,11078ms | completed |
| vlun4-r1-native-06d7-2032-d | stalled2031-12-01 | 0 | no return | not completed |
| vlun4-r1-native-nosave-06d7-2032-e | clean2032-01-01 | 0 | omitted control | completed |
| vlun4-r2-native-noprobe-save-f | stalled2031-12-01 | 0 | no return | not completed |
| vlun4-r2-current40-save-g | clean2032-01-01, failed ref criterion | 4 | returned1,11031ms | completed |
| vlun4-r2-current40-noprobe-h | clean2032-01-01, failed ref criterion | 4 | returned1,11016ms | completed |

All unique completed verdicts are retained. Seven full-save attempts: four returned, three stalled. No failed-start/harness-error retry, refusal, label reuse or discarded verdict occurred this round. Every executed corrected probe has strict116235/116235, host2484/4140, gaps2629/3043, zero acquisition/storage refusals, final native/core release0 for all46, managerUnchanged1 and no resync. The two no-probe controls execute zero comparisons and receive zero verification credit.

### Remaining conditions

Native full-save/full-shutdown remains failed; current40 ref0 remains failed. The successful corrected owned full-save2032/ref0 and corrected native no-save2032/ref0 from round1 are retained without repeating them. The harness documents `fixture_city` import from main `build-runs/fixtures`, but no export request or supported save-as action for these runs. No fixture from these immutable runs was supplied for reload. Original/owned/current40 reloads are **not exercised**, and no reload credit is claimed. No unsupported save-as, slot copying, game start, Documents access, or arbitrary fixture substitution was attempted.

No new spec question was filed in round2. **LU4** remains in `docs/specs/questions/language-utility.md`; **LU1-LU3/SL1** remain unresolved. C ABI exports are unchanged this round; the lane as a whole retains the three private transport exports described below. No readiness or owner-completeness claim is made. At most one lane round remains under the original three-round cap; round2 used three justified new requests, without resetting any count. No actual lane-guard denial occurred. Round2 milestones: initial handoff8e07c52a, native isolationeca36fe7, current40 result/final gatea16c81ad; detached HEAD retained throughout.

## Retained round1 result and scope

An opt-in constructed-native probe now compares independently owned ORIGINAL utilities with the actual production `fc4_core::language::LanguageUtility`, through a private persistent CAPI transport. Set `language_native_verify = 1` to run it at PostAppInit. The production utility implementation and its name tables are unchanged. String, detokenizer and encoding owners, defaults, shared tools/maps and ABI tables are untouched. No whole-owner verification, replacement or promotion is claimed.

Source milestones: safe-path record `b4c8ac8e`; implementation `0641be7e`; audit/gap correction `06d71bb2`. The worktree was reserved at detached HEAD and stayed detached. No merge/rebase/push, subagents, executable-byte inspection, private CRT binding, or game-file commit occurred.

## Acquisition, ABI, storage and lifetime

The safe path was established before call 15, and source was written by call 20. The SDK provides language-manager service `0x441E5070`, IID `0x0054B7D5`, and `GetNewLanguageUtility(id)` at slot 39. The spec gives each returned new utility one caller-owned reference. SDK utility slots 0..29 supply typed calls; published ABI slots 31/32 and the spec's void return supply setters on these owned objects only.

Output, format and symbol storage uses the existing version-guarded `fc4::GameString` and B allocator, with the published cRZString layout and matching allocation/free policy. No SDK A-buffer string crosses a native regrow/free boundary, and no string owner was changed. No new executable binding was added.

The manager service hold is audited and balanced. Its embedded utility is borrowed only for read-only before/after controls, never released or mutated. Created utilities are recorded with `FC4_ACQUIRED` and destroyed with `FC4_RELEASE_CREATED`; extra AddRef and successful QI references are separately balanced. Every final native/core Release returned 0 across all 46 objects. Current language, current country and borrowed utility identity/driving-side control stayed unchanged. No process/global locale setter is called.

The first probe omitted audit registration of the already-owned creation hold; its 359 warnings are retained. The correction adds audit accounting, without changing native AddRef/Release behavior or production core behavior. The corrected clean runs have ref_warnings=0.

## Measurements and limits

Each corrected run compares 46 independently constructed native/core object lifetimes, IDs 0..0x2D. The core object persists through its sequence. No native result is used to create an input, seed a core table, refresh state or supply a formatting answer. Resync=0, utility fallback=0 and acquisition/storage refusal=0.

- Normative constructed checks: **116235/116235 exact** per corrected run.
- Name checks: **96784/96784 exact** across all IDs, four methods, every byte index 0..255 and high-byte indices 256, 257, 263, 268, 511, 0x10001 and 0xFFFFFFFF, each with empty and nonempty initial output. This includes stored invalid UTF-8/control bytes, missing entries and low-byte behavior. ID0 results remain conditional on SL1 rather than receiving normative construction credit.
- Queries cover currency/separators, driving side, measurement, name order, IME, multibyte and 18 charset queries. Six country values exercise setters/measurement. These are bounded samples, not all u32 countries/code pages.
- Formatting covers positive/negative/zero integers through INT64_MAX, default/custom/empty symbols, time normalization and negative components, date styles 0..6/UINT32_MAX, several month/year boundaries, finite float formats, invalid formatted-money conversions and escaped percent. INT64_MIN and nonfinite doubles are not claimed.
- QI checks both accepted IDs plus three rejected IDs and untouched failure output; AddRef/Release, Init, owned setters and shutdown observations are exercised.
- Case/compare use independently constructed ASCII inputs and multiple entry lengths. No exhaustive ANSI/Unicode conversion or collation claim is made.
- Natural calls: **0/0, not exercised, no credit**. There are no interception hooks.

`LocaleHost` is explicitly foreign: Windows ANSI APIs plus the plugin DLL CRT's snprintf. It never calls the native utility under comparison, guesses its private CRT, or substitutes a native formatted answer. Host-dependent formatted checks are **2484/4140 exact**, with **1656 raw differences** retained. They include signed-zero spelling and two-versus-three-digit exponents, not normative exactness for that boundary.

Open-question observations are **2629/3043 exact**, with **414 differences**: 46 LU1 negative-year cases and 368 LU2 case-return cases. The case output bytes match. LU3 shutdown and SL1 ID0 construction remain unanswered even where observations match. LU1-LU3/SL1 bracketed annotations were never treated as normative contracts. The first probe incorrectly classified 270 LU2 return differences as normative; corrected source moves those checks to the explicitly counted gap category, preserving original logs and counts. Production behavior was not fitted to these observations.

Every raw difference is logged with input values, initial output bytes, format/symbol bytes, native/core return values and both outputs in hex. No differences were suppressed. Turbo and normal-speed runs reproduce the same utility counts.

## Immutable game evidence

All requests use unique labels and one outstanding request at a time. Waiting used lane-local `build-runs/wait-language-verdict.py`, not qwait. All five verdicts/logs remain under `build-runs/game-results/`; none was discarded or relabelled.

First DLL: `build-runs/language-immutable/FreeCity4.dll`, SHA256 `e0bc89a7cffde639cb337380ac0057cbd13a95ab61127b2f54120d2300ba463b`.

- `vlun4-r1-own-0641-2032-a`: clean through 2032-01-01, **ref_warnings=359**; full save/full shutdown completed. Initial counters strict116235/116505, host2484/4140, gaps2629/2773. This is retained failed reference-audit evidence, not a passing run.
- `vlun4-r1-native-0641-2032-b`: original simulator/budget, normal speed; identical initial utility counts, then **stalled (fallback)** at scheduled 2031-12 save. No successful save/full-shutdown/final ref0 claim.

Corrected DLL: `build-runs/language-immutable2/FreeCity4.dll`, SHA256 `78ea9b099f55d8abd95ea76cea21f34ced2caf72fc68ec305f35da43185c9342`. The final-gated engine DLL has the same hash.

- `vlun4-r1-own-06d7-2032-c`: own simulator/original budget, turbo, **clean/ref0** through 2032-01-08; full save returned1 in11078ms, PostAppShutdown completed. Corrected counters strict116235/116235, host2484/4140, gaps2629/3043, objects46, all finalRelease0/0, managerUnchanged1.
- `vlun4-r1-native-06d7-2032-d`: original simulator/budget, normal speed; identical corrected utility counts and zero reference warnings in its log, then **stalled (fallback)** at scheduled 2031-12-01 save. No completed native full-save/full-shutdown/final ref0 verdict.
- `vlun4-r1-native-nosave-06d7-2032-e`: same corrected DLL, original simulator/budget, normal speed, scheduled save omitted to isolate that boundary: **clean/ref0** through 2032-01-01, PostAppShutdown and PostSystemServiceShutdown completed; identical corrected utility counts. This isolates the two save failures; it does not erase them or prove native full save.

At the end of round1 no request remained outstanding. The exact coordinator `current40` enable profile was not then available; round2 above uses the subsequently supplied literal40 list. No guessed list was credited. No unsupported saved-city reload or Documents access was attempted.

## Build gate and formatting

Focused qbuild quick gates passed: capi530/0 with builds211/211 and4/4. One premature full-gate attempt was interrupted before CTest after the first game exposed audit/gap instrumentation issues. After the correction, the single completed stable `qbuild all` passed **cargo4961/0, clippy0 diagnostics, CTest68/68**. Full logs are `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log` and `fc4verify-ctest.log`.

The new Rust file passes qbuild fmt. The requested recursive fmt check of touched lib.rs reports645 existing diff lines in other modules, starting advisors.rs; a direct skip_children check also reports pre-existing lib.rs span565_tests ordering. Those files/sections were not reformatted. The wrapper rejects a skip_children option. Initial q.py access to WSL was denied; only the new Rust file was then formatted directly. No unchanged full baseline build or mass reformat was performed.

## Questions and remaining work

New **LU4** is appended to `docs/specs/questions/language-utility.md` for the observable foreign printf boundary and signed-zero/exponent differences. Existing **LU1, LU2, LU3** and **SL1** remain open and separately counted. The generated question index was not edited or regenerated with prohibited tools.

Round1 ended WIP: the exact current40 control was then missing and both native full-save controls failed. Round2 above supplies the literal control and the no-verifier native baseline, retaining every earlier failure. Reconcile native save/setup, current40 shutdown reference warnings, and supported fixture reloads; obtain published answers for LU1-LU4/SL1 before expanding credit. Regenerate the question index through the authorized coordinator workflow. Do not infer owner completeness or change defaults from the exact bounded constructed checks.

C ABI change: three private transport symbols (`fc4_language_verify_new`, `fc4_language_verify_call`, `fc4_language_verify_free`); no public header/ABI-table edits or ABI-version bump.

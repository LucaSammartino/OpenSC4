| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-language-native4 (native utility verification) | `Codex` / `verify-language-native4` (engine lane; detached HEAD) | in progress: round2; source06d71bb2; prior cargo4961/0, clippy clean, ctest68/68; owned/native-no-save2032 clean/ref0; strict116235/116235; two native full-save stalls retained | Diagnose identical-script no-verifier native full-save control, then corrected literal current40; retain LU1-LU4/SL1 gaps. |

## Handoff

- Goal: round2 of at most3; diagnose full-save stall before any owned fix; exact supplied house40 control.
- Done: prior round report at35a599ca; source06d71bb2; all five prior attempts and raw counts retained below.
- Baseline: round2 qbuild all complete; cargo4961/0, clippy clean, CTest68/68; DLL hash unchanged.
- Done: vlun4-r2-native-noprobe-save-f stalled2031-12-01/ref0; identical immutable2 DLL/script/native profile, only verifier switched off. No language native BEGIN/END or callback executed.
- Diagnosis: owned PDB resolves all eight plugin stack candidates identically to prior probe-on stall: observer/script SaveCity pending; no language probe frame. Watchdog kills at10s without OnTick. Native baseline/setup failure reproduced, no owned language fix justified.
- Next: run literal current40 profile full-save control; preserve native failures. Reload requires supported main build-runs/fixtures staging, not arbitrary save_as.
- Facts: corrected immutable2 DLL SHA25678ea9b099f55d8abd95ea76cea21f34ced2caf72fc68ec305f35da43185c9342; no prior request outstanding.
- Strict116235/116235; host2484/4140; gaps2629/3043. No production algorithm fit is permitted.
- Current40 literal supplied in round2 prompt; excludes terrain/dispatch/effects. Preserve every failure, no unchanged failed-setup repeat.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-verify-language-native4-r2-manifest.json.

## Result and scope

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

No request remains outstanding. The exact coordinator `current40` enable profile was not found in permitted sources. No guessed list was submitted or credited as that control. No unsupported saved-city reload or Documents access was attempted.

## Build gate and formatting

Focused qbuild quick gates passed: capi530/0 with builds211/211 and4/4. One premature full-gate attempt was interrupted before CTest after the first game exposed audit/gap instrumentation issues. After the correction, the single completed stable `qbuild all` passed **cargo4961/0, clippy0 diagnostics, CTest68/68**. Full logs are `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log` and `fc4verify-ctest.log`.

The new Rust file passes qbuild fmt. The requested recursive fmt check of touched lib.rs reports645 existing diff lines in other modules, starting advisors.rs; a direct skip_children check also reports pre-existing lib.rs span565_tests ordering. Those files/sections were not reformatted. The wrapper rejects a skip_children option. Initial q.py access to WSL was denied; only the new Rust file was then formatted directly. No unchanged full baseline build or mass reformat was performed.

## Questions and remaining work

New **LU4** is appended to `docs/specs/questions/language-utility.md` for the observable foreign printf boundary and signed-zero/exponent differences. Existing **LU1, LU2, LU3** and **SL1** remain open and separately counted. The generated question index was not edited or regenerated with prohibited tools.

WIP, not merge-ready: the required exact current40 control is missing and both native full-save controls failed. Coordinator next steps are to provide that literal profile, reconcile the native save/control boundary, and obtain published answers for LU1-LU4/SL1 before expanding credit. Regenerate the question index through the authorized coordinator workflow. Do not infer owner completeness or change defaults from the exact bounded constructed checks.

C ABI change: three private transport symbols (`fc4_language_verify_new`, `fc4_language_verify_call`, `fc4_language_verify_free`); no public header/ABI-table edits or ABI-version bump.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-language-native4 (native utility verification) | `Codex` / `verify-language-native4` (engine lane; detached HEAD) | Ready within bounded constructed utility scope; FINAL3/3; source06d71bb2 unchanged; retained cargo4961/0, clippy0, CTest68/68; r3 i/j/k clean/ref0 full-save/full-shutdown; ON j/k strict116235/116235 each, host1656/gap414 differences each; actual42=40+2 defaults; all8 historical attempts retained; LU1-LU4/SL1 open; lane retains3 private ABI exports | (1) Published answers for LU1-LU4/SL1 before expanding credit; (2) reload only if a supported isolated fixture becomes available; no fourth lane round, owner/default/replaced promotion or further experiment. |

## Final round3 report (2026-10-22)

**Ready for the bounded constructed utility verifier and the three explicit supported lifecycle controls below.** This is the final original-family round3/3. No engine source, core behavior, provider, CRT, locale, lifetime, unrelated owner, default, public ABI table or generated entry was changed this round. No fourth or renamed continuation is requested. The historical reports below retain their state and conclusions as of those earlier rounds; this report adds the supported-profile results without rewriting their failures.

### Artifact and retained stable gate

Before the experiment and again after all three verdicts, the actual committed engine tree equals `06d71bb2:engine`: `b5044770680594c46e936d71a4b5bec83a63c10f`. There are no working or untracked engine changes. The built `engine/build-x86/FreeCity4.dll` and this lane's `build-runs/language-immutable2/FreeCity4.dll` both hash to **78ea9b099f55d8abd95ea76cea21f34ced2caf72fc68ec305f35da43185c9342**. Both PDB copies hash to `ab9ee7606d4ea62a51ab6bccc142bc989264334c55aab2dfbd384ef4cd406786`. All three new harness verdicts record actual DLL hash prefix `78ea9b09`. No cmdline-lane DLL was used.

The audited retained stable gate is **cargo4961 passed/0 failed, clippy0 errors, CTest68/68** (retained CTest log93.03s). Following this round's unchanged-source instruction, no redundant qbuild all, focused source check or Rustfmt was run. Gate logs and their SHA256 values remain:

| Lane-local log | SHA256 |
|---|---|
| build-runs/qbuild-test.log | 3d008cfe487b3a5bc5dc8a5187c7f20f1f169565317c4e85b686f9932e56be72 |
| build-runs/qbuild-clippy.log | dba89f1c7ac2b84fddbb8114ea2f767e31defb34d36231ba1fafa66c492a365e |
| build-runs/fc4verify-build.log | d86b00917a1b2b8de291b35e8ae963baaca9639586b29b929c5e9e85aea413d0 |
| build-runs/fc4verify-ctest.log | 88d42603acdd8c7825982b6ee2a16f135c3a2b800222e3f33b9620f4e11df13b |

### Controlled supported profile

All three requests used **Konradshohe, seconds420, timeout600, turbo1, end_date2032-01-01**, the same immutable2 DLL, `ref_audit=1`, popup dialogs off, and full quit. They used the already copied lane-local `build-runs/evidence-run4/cmdline4-r3-supported-save.script` unchanged: `2031-12-01 save`, then `2031-12-15 mark persist-control-fullsave-complete`. Its SHA256 is `70a903f02158b86981fbc3d5411c8e2ace5a9ea6539b5ab3806f0ae07fa24ac6` before and after the experiment.

The copied cmdline round3 controls demonstrate this harness setup with a different artifact; they establish no language compatibility or failure cause. The prior own-DLL `vlun4-r1-own-06d7-2032-c` already passed the equivalent effective simulator-only/budget-disabled/turbo profile with full save/ref0. This round adds an OFF-first matched pair on the precise420/600/save+mark profile; it is not first-ever proof of that effective profile. Historical language native-fullsave failures all used disable-all/normal-speed, and remain failed evidence for that distinct setup.

OFF and ON request dictionaries differ **only in label and language_native_verify (0 versus1)**. Both have empty enable and `disable=budget`. Their actual production enabled set is exactly **simulator-only (1)**: our simulator supplies the harness clock, budget is disabled/original, and optional owners remain original. This is not an all-original simulator run or owner binding.

CURRENT40 changes only label/enable/disable relative to ON. Its enable value is exactly these40 distinct literal keys:

`landvalue,weather,ordinance,demand,power,plumbing,flammability,pollution,protection,crime,aura,flora,landfill,residential,neighbors,zones,development,tract,history,clock24,commercial,industrial,civic,parks,lotdev,zonedev,traffic,automata,netmgr,netlot,nettool,demolition,occmgr,propmgr,lotcfg,citysit,propdev,simgrids,citydetail,ports`

With disable empty, its actual logged enabled set is **42 = 40 requested + 2 defaults (simulator, budget), extra0/missing0**. Terrain/dispatch/effects are absent. Forty literal keys are not relabelled as42 keys, and42 actual providers are not relabelled as40 providers.

### New raw lifecycle results

All timestamps below are the log's2026-10-22 timestamps; SaveCity timestamps are the completion log lines. All saves returned `ok=1` for `SaveCity(full)`. The mark completed in all three runs. Each reached both full shutdown hooks and the final clean verdict with **ref_warnings0, foreign_exceptions0, watchdog_stalls0, rtc0, overruns0**.

| Unique label | Probe / actual providers | Verdict date | Full save completion / duration / execution date | PostAppShutdown / PostSystemServiceShutdown |
|---|---|---|---|---|
| vlun4-r3-supported-off-2032-i | OFF / simulator1, budget disabled | clean2032-01-04/ref0 | 22:10:06.574 /10953ms /2031-12-16 | 22:10:07.222 /22:10:07.302 |
| vlun4-r3-supported-on-2032-j | ON / simulator1, budget disabled | clean2032-01-24/ref0 | 22:12:51.795 /11079ms /2031-12-14 | 22:12:52.524 /22:12:52.597 |
| vlun4-r3-current40-budget-2032-k | ON / literal40+simulator+budget=42 | clean2032-01-01/ref0 | 22:16:28.844 /11093ms /2031-12-01 | 22:16:30.940 /22:16:31.089 |

Turbo can execute dated actions after their scheduled day and overshoot the requested end date; the actual execution/final dates are retained. Session seeds and simulation trajectories are not asserted identical. OFF executed zero language comparisons and receives zero verification credit.

**Raw warnings are retained, not treated as zero:** OFF/ON/CURRENT40 contain4/4/8 WARNING lines respectively. Each contains the two expected x87 startup warnings. OFF and ON each also contain one `config: WARNING unknown key 'language_native_verify' in FreeCity4.ini (ignored)` and one corresponding `ref_audit` warning; CURRENT40 contains three of each config warning across its reloads. The corrected historical OWNc log already contains these same config warnings. The immutable source reads the language flag directly with `SettingInt`, and ON's actual46-object execution confirms it took effect despite the central known-key warning. The correctly named reference-audit setting is `refaudit`, not the request's unrecognized `ref_audit`; the latter receives no enabling credit. Reference auditing is on by default, and actual `refaudit: on` log states confirm it remained active: one reload state each for OFF/ON, three for CURRENT40. Ref0 is the active auditor's result, not a disabled audit. No key/default/source change or rerun was made to remove these diagnostics.

On **each** new enabled run j/k, actual production `fc4_core::language::LanguageUtility` versus independently owned ORIGINAL utility comparisons are **strict116235/116235**. Separate foreign `LocaleHost` observations are **host2484/4140,1656 differences**; separate open-question observations are **gaps2629/3043,414 differences**. All2070 raw DIFF lines per enabled run remain in its log. Exact strict results do not erase those differences or make the foreign host normative.

Each enabled run constructed46 caller-owned utilities, IDs0..45, with46 final native/core Release results0/0, refused0, resync0, fallback0 and managerUnchanged1. The borrowed manager utility was neither mutated nor released, the acquired service reference and extra AddRef/QI holds were balanced, and no global locale switch or foreign/private CRT binding was added. ID0 construction/results remain conditional on **SL1**. Natural calls remain **0/0, unexercised**. No arbitrary parser/codec, string, detokenizer, encoding or manager-owner completeness is claimed.

The two new ON runs repeat the same bounded corpus: **232470 strict comparisons = 2 x116235**,92 constructed lifetimes over the same46 IDs. They add stability/lifecycle observations, not twice the unique input coverage. Their combined raw foreign-host3312 and question828 differences remain separately classified. **LU1-LU4/SL1 all remain open**: LU1 negative year46 and LU2 case return368 account for414 differences per run; LU3 shutdown and SL1 retain conditional observations even where matched; LU4 retains the foreign printf/signed-zero/exponent boundary. No new question was filed this round.

The budget-enabled current42 result passes the required ref0 control, while both historical budget-disabled current40 controls remain clean/ref4 failures of that criterion. This is compatibility evidence for the tested supported profile, not a causal diagnosis of the old warnings or stalls. Timing/script and simulator/budget setup differ across the historical profiles. No budget/ports/parks/landfill fix, language behavior fitting, owner promotion or default change follows from this experiment.

### Evidence retention and final limits

All eight historical attempts remain below and in lane-local raw artifacts: the359 audit warnings, all three full-save stalls, the omitted-save control, both current40/ref4 controls and the corrected OWNc success. Together with the three unique round3 requests this is **11 retained attempts:10 full-save attempts (7 returned,3 stalled) and1 omitted-save control**. No label reuse, discarded completed verdict, reroll, failed-start/harness-error retry or fourth-round variant occurred. No request remains outstanding. Each wait was launched once with the existing lane-local waiter accepting an already completed verdict; no qwait freshness reset or process-completion polling was used.

| New label suffix | Request SHA256 | Raw verdict SHA256 | Raw log SHA256 |
|---|---|---|---|
| supported-off-2032-i | 62fac407f1f17e90118a3003861e0bd73a641e047d87464fa7e578a81b9834c7 | 6287c403b90a7c46698418dab76233e078a313c1598824350649224a3f3e1fe5 | 36e78eb55aaf0e935cf6f333764744c517d565163a2e0e39bc05c1f1eed29c45 |
| supported-on-2032-j | d0b5103bd5b069cb4bf675d73153344995ed2a9b20b9ea9fa5058bea2a5a0cd5 | f17efbfe7dc2fbc62917b4f97a1da619671bb29b9cbab70187a7d2d15d5751f4 | 0a4a46b40d64ff832aa586737c2e22cb942b0f22a1182032c27f6aacbdc1490a |
| current40-budget-2032-k | 1bc5fc18413afbe8198ab89e0f5b0894742da62a43a68cb5cb7e9801a1bd591b | 346fd87e031ead7c45bb60f0fea9675560caea3aad947578d04d945d57e30594 | d9cc165186b43307c5389a6f25e5d58dba23c0cfd0dda0a2ea9a702748a9cdce |

Full labels have prefix `vlun4-r3-`; requests are retained under `build-runs/game-queue/done/`, raw verdicts/logs under `build-runs/game-results/`. Read-only checks are reproducible with `python build-runs/audit-language-round3.py gate`, `pair`, any of the full new labels, or `warnings` followed by those labels. The q.py wrapper's Bash transport was unavailable (access denied); bounded lane-local Python was used directly. No prohibited source, private executable, other worktree, game slot or Documents file was inspected. No subagents, tools/maps edits, shared refs/config edits, merge/rebase/push or external communication occurred. Commits stayed detached; git's post-commit packed-refs lock attempt was denied by the read-only shared-ref boundary, while each commit itself succeeded.

No already available supported isolated fixture from these immutable runs was supplied. Original/simulator-only/current42 **reloads remain unmeasured**; no fixture lookup outside the worktree, slot copy, save-as/export guess or Documents access was attempted. Published answers in `docs/specs/questions/language-utility.md` (LU1-LU4) and `docs/specs/questions/strings-language.md` (SL1), and an authorized supported fixture, are prerequisites for any later scope expansion. Bounded constructed utility/lifecycle readiness gives no full-owner/default/replaced or arbitrary parser/codec credit.

Round3 milestones: handoff9ca61786, provenance/gate auditdf3e80ef, OFF86fee1c2, ON2ef73713, pending-label ledgerdfc0b29e. Final reporting retains source milestone06d71bb2. This round changes **no C ABI exports**; the lane as a whole still adds the three private transport exports listed in the retained report, so the merge-queue/result ABI flag remains **true**. No public ABI-table edit or ABI-version bump.

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

## Coordinator integration, 2026-10-22

After the final third round, the coordinator merged main `7e0dd45c` into this completed branch without
launching another worker round. Three conflicts were additive: retain the language source pair alongside
FontSys/span8/feature/persistence CMake entries, retain every CAPI module, and retain language plus
span8/persistence startup calls. The language verifier source files are unchanged from `223d0644`.
The retained 4961/clippy0/68 gate and immutable DLL prove the historical worker artifact only; this integrated
engine tree requires the next merge train's real gate and regression. No new game comparison or broader
native/owner/default credit comes from this mechanical integration. Earlier round1/round2 residual text
above remains historical; the final round3 controls and limitations are recorded at the top of this report.

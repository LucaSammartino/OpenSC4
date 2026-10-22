| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-cmdline-native4: public native read-only queries | `verify-cmdline-native4` / detached worktree | in progress: final round3 supported-profile lifecycle validation | Check immutable-b correspondence, then matched native probeOFF/ON and conditional current40 controls; stop on OFF failure. |

## Handoff

- Goal: final3/3 bounded lifecycle continuation of borrowed production/native queries; CL1-CL4 exclusions persist.
- Reused detached HEAD088f78a2, initially clean. No production changes planned; immutable-b source92e01745, SHA256053BD7A077A43A31ACB065C3B985D792CD9FEBFADAA82B0E7A343D742621FABC.
- Copied successful control: build-runs/evidence-run4/vpm4-r1-native-f875545f.{req,log,verdict}; script saves2031-12-01, mark2031-12-15, end2032-01-01; enable empty, disable budget, simulator retained. This is a hypothesis to test, not a cause finding.
- Confirmed engine diff92e01745..HEAD empty; engine/build-x86 and immutable-b DLL hashes both053BD7A077A43A31ACB065C3B985D792CD9FEBFADAA82B0E7A343D742621FABC. Stable33596de3 records final source gate cargo4963/clippy0/ctest68; logs retained, DLL19:46:52 precedes gate completion19:49:01. No rebuild justified for unchanged source.
- Source inspection confirms independent GetCommandLineA -> production CAPI/Rust parser, actual borrowed public SDK dispatch, no native seeding or clone. Same fixed query inventory and exclusions as round2.
- NativeOFF cmdline4-r3-native-off-2032-e CLEAN2032-01-07, full save ok1 at real2026-10-22 21:19:10.807/game2031-12-09,10984ms, PostSystemServiceShutdown, ref0/stalls0. Probe disabled: no comparisons/coverage credit. Actual enabled set simulator only; budget disabled; optional owners original.
- NativeON cmdline4-r3-native-on-2032-f CLEAN2032-01-04,123/123 exact/failures0/differences0,argc5/raw174; full save ok1 at real2026-10-22 21:22:27.908/game2031-12-03,11266ms; shutdown/ref0/stalls0. Actual enabled simulator only; otherwise matches OFF. No source change or causal claim about old failures.
- Outstanding: unique cmdline4-r3-current40-2032-g, running slot1 since2026-10-22 21:24:31; SAME immutable-b DLL/city/script/timing, cmdline_probe1, exactly40 specified literal enable keys, disable empty. Await existing request/verdict without replacement; attribute simulator/budget defaults separately and check actual42-entry set when complete.
- All round2 disable-all stalls and prior current40 evidence retained below and in build-runs. Next: audit this unique current40 result, record every failure if present, finalize bounded ready only on clean full-save/shutdown/ref0.
- No source changes, no new questions, no new gate; task explicitly permits inspecting existing stable gate for unchanged source.
- q.py Bash transport failed with E_ACCESSDENIED; use bounded PowerShell reads, apply_patch for all files.
- Audit script build-runs/cmdline4-r3-audit.py gate recomputes4963/0/0 and exact hashes; run audit corrected empty-enable whitespace parsing before recording configuration counts. Completed OFF verdict accepted without resubmission.
- ON method op0..9 exact/total1,6,1,12,12,16,16,14,24,21; present/absent op3..9=10/2,10/2,6/10,4/12,6/8,2/22,2/19. Audit failure matcher corrected to exclude the successful failures=0 summary. No .sc4 fixture exists in this worktree's build-runs; replay not exercised or claimed.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-verify-cmdline-native4-r3-manifest.json. No fourth/renamed round, external reads, agents, ref updates or private owner work.

## Round 3 evidence (2026-10-22; final control pending)

The final bounded continuation has passed matched **native optional-owner profile
OFF and ON** lifecycle controls with the retained immutable-b DLL. Both use
Konradshohe, turbo1, seconds420/timeout600, end2032-01-01, the same save2031-12-01
and mark2031-12-15 script, enable empty and disable budget. Only the label and
cmdline_probe setting differ. The simulator remains our enabled harness; budget
is disabled and all optional owners remain original. This is the supported explicit
profile copied from the persist control, not an all-original simulator claim.

| Unique label | Probe / exact queries | Verdict / date | Full save actual timestamp (game date); duration | Shutdown / ref warnings / stalls |
|---|---|---|---|---|
| cmdline4-r3-native-off-2032-e | OFF; disabled, no coverage credit | clean /2032-01-07 | 2026-10-22 21:19:10.807 (2031-12-09);10984ms,ok1 | PostSystemServiceShutdown /0/0 |
| cmdline4-r3-native-on-2032-f | ON;123/123, differences0/failures0 | clean /2032-01-04 | 2026-10-22 21:22:27.908 (2031-12-03);11266ms,ok1 | PostSystemServiceShutdown /0/0 |

### Source, artifact and stable gate correspondence

Production source stays **92e01745**. `git diff --quiet 92e01745 HEAD -- engine`
returns0. Both `engine/build-x86/FreeCity4.dll` and the artifact used by every new
request, `build-runs/cmdline4-r2-immutable-b/FreeCity4.dll`, hash to
**053BD7A077A43A31ACB065C3B985D792CD9FEBFADAA82B0E7A343D742621FABC**.
The retained stable full gate was recorded by33596de3 after92e01745: workspace
**4963 passed/0 failed/0 ignored**, clippy **0 diagnostics**, build **2/2**,
ctest **68/68**,72.88s. DLL timestamp19:46:52 precedes gate completion19:49:01.
Audit recomputes the cargo totals and hashes from actual retained files. Final
source's preceding quick gate was capi532/0 and C++5/5. Earlier baseline/gate logs
remain separately in build-runs/cmdline-r2-baseline-* and cmdline-r2-gate-a-*.
As explicitly directed for unchanged source, these exact retained checks replace
a redundant full build in round3; no Rust files were touched or reformatted.

### Accepted query contract and exclusions

Each enabled probe independently captures GetCommandLineA bytes, constructs the
actual production `fc4_core::cmdline::CmdLine` through the CAPI once, and compares
its outputs to actual typed public SDK calls on the framework's borrowed original
CommandLine. Input, query bounds and natural Find keys come from the independent
capture/production parse; native outputs never seed state or expected answers.
The initialized framework has a balanced acquired hold; the borrowed command-line
pointer is neither AddRef'd nor released. No clone oracle, resynchronization,
fallback, native mutation, constructor, allocator or private lifetime call is used.

New native ON logged argc5 and raw174 bytes. Its accepted method inventory is
exactly the ten read-query variants in the table below (the same counts as the
retained round2 table). Raw argc, argv0..5 including the empty out-of-range result,
and full command-line text compare exactly. The process shape is program path,
`-CPUCount:1`, `-w`, `-intro:off`, `-UserDir:` with the isolated slot path.
Fixed ASCII names intro,CPUCount,CustomResolution,r,d,f,w and an absent marker,
uppercase forms, chars iCrdfwZ, both case/unquote flags as applicable and starts
-1/0/argc exercise the published read methods. Each query compares scalar,
string length and bytes in process; logs retain raw natural outputs, per-method
exact/positive/negative totals and any failed tuples. Successful search/switch
tuples are aggregated rather than dumped individually; no extra raw fixture
coverage is inferred from those totals.

CL1 private slot16, CL2 empty switch names and CL3 high-byte/locale behavior remain
excluded. Unsafe negative argv/erase, exception-prone single-quote input, owned
parse/copy/mutator/lifetime domains are not exercised; CL4 remains open. Constructed
original fixtures are not exercised and earn no0/0 credit. New queries are repeated
natural process-query evidence, not independent arbitrary parser fixtures or all17
owner methods. The opt-in defaults off; no owner or default enablement promotion.
The lane's earlier three CAPI exports remain an ABI change, with ABI version84
unchanged; round3 makes no engine/export/spec/map/table/default edits.

### Preserved failures and replay limits

All round2 disable-all native stalls and the earlier current40 clean control are
preserved in their original requests/logs/verdicts and the historical report below.
Different profile success is measured; it does **not** establish why disable-all
stalled. The copied persist control used another DLL and is only the profile/timing
prerequisite, never cmdline comparison evidence. The new matched current40 request
uses this lane's same immutable DLL and will be reported separately.

No `.sc4` fixture exists in this worktree's build-runs. Documented saved-city staging
requires an actual isolated saved fixture; no supplied fixture is available within
this lane's permitted evidence. Full save completion is proved, but reload/replay,
saved-state byte equivalence and future-load behavior are unmeasured. No slot or
Documents game file was accessed, exported or committed. Queries show no difference,
so no turbo-only difference requires a normal-speed replay. No new spec question or
engine correction is justified. CL1-CL4 remain the existing question IDs in
docs/specs/questions/command-line.md.

## Round 2 report (retained historical outcome, 2026-10-22)

The public original endpoint is implemented and exercised: **492/492 read-only query comparisons exact across four launches**, each using the same five-argument process shape. This is bounded natural process-query evidence, not 492 independent fixtures or full owner verification. The required native 2032 full-save/shutdown control did not pass, so the lane remains **wip**. No requests remain outstanding.

### Implementation and ownership

- `engine/plugin/FC4CmdLineProbe.h`, called synchronously at the end of the director's PostAppInit, uses its initialized public `FrameWork()` pointer. An audited framework AddRef/Release pair keeps that hold alive throughout the probe. Public typed SDK `CommandLine()` returns the borrowed original; it is never AddRef'd, released, initialized, shut down, mutated, or deleted. No private slots, raw object writes, addresses, allocator, constructor, or owned native copies are used.
- Independently capture full `GetCommandLineA()` once per enabled process, as command-line.md section 5 specifies for Windows NT. The new private CAPI transport creates a real production `fc4_core::cmdline::CmdLine` once from those bytes and calls its actual parser/query methods. Native outputs are compared and logged; they never build Rust input, state, or answers. No resynchronization, native forwarding, or substitute parser.
- The opt-in `cmdline_probe = 1` defaults off. Its key is appended to FC4Config's recognized-key list; existing defaults are unchanged. The first run's unknown-key warning was corrected by this registration. New CAPI symbols are `fc4_cmdline_probe_create`, `fc4_cmdline_probe_query`, and `fc4_cmdline_probe_destroy`: **ABI exports changed**, but ABI version84, fc4.h, ABI tables/maps and generated abi_entries.inc are unchanged.
- Actual argc5, argv order/text (including argv(argc)'s empty result), original full text174 bytes, Find/FindSubString, and named/char presence/value/index queries are compared. Query bounds and natural Find keys come only from independently parsed Rust input. Fixed ASCII names and absent names exercise positive/negative queries, case flags, unquote flags, untouched output strings, and index starts -1/0/argc.
- Public route source was established in the initial 15-call audit; implementation committed `adb42d1d` before call30. Formatting/focused gate: `07aa4988`. Final logging/key source: `92e01745`; full gate recorded `33596de3`; current40 verdict recorded `43259b84`. Detached HEAD was retained; no refs switched, merged, rebased, or pushed.

### Per-process production-method evidence

The following exact counts were logged by both native-b and current40-b. Other enabled-probe launches also logged123/123 exact. Positive/negative counts describe native return values, not generated expectations.

| Public read method | Exact/total | Present / absent |
|---|---:|---:|
| argc | 1/1 | - |
| argv | 6/6 | - |
| GetCommandLineText | 1/1 | - |
| Find | 12/12 | 10 / 2 |
| FindSubString | 12/12 | 10 / 2 |
| IsSwitchPresent(name) | 16/16 | 6 / 10 |
| IsSwitchPresent(name, value, unquote) | 16/16 | 4 / 12 |
| IsSwitchPresent(char, value, unquote) | 14/14 | 6 / 8 |
| GetIndexOfSwitch(name, start) | 24/24 | 2 / 22 |
| GetIndexOfSwitch(char, start) | 21/21 | 2 / 19 |

Constructed native fixtures0/0, not exercised. Original command-line references acquired/released0/0; framework hold balanced per enabled probe. Native mutations0, fallbacks/resynchronizations0, query transport failures0, query differences0. CL1 private extra slot, CL2 empty switch name, and CL3 high-byte case behavior remain excluded. CL4 owned construction/mutators/lifetime is still open: this phase does not answer it or prove all17 owner methods.

### Gates and retained failures

- Final source `92e01745`: qbuild quick PASS, capi532 passed/0 failed, C++5/5 edges. Stable qbuild all PASS: cargo workspace4963 passed/0 failed/0 ignored; clippy0 diagnostics; C++2/2 edges; ctest68/68,72.9s. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. No engine edits followed this gate.
- Earlier stable all at `07aa4988` also passed cargo4963/clippy0/ctest68/68 (157s). Initial baseline all passed its unchanged Rust/clippy stages, but C++ compilation overlapped my first incomplete hook edit and failed on the missing include; this is a retained failed attempt, not a pristine passing baseline. Its logs are `build-runs/cmdline-r2-baseline-{test,clippy,build}.log`.
- qbuild fmt on the new CAPI file PASS. The first relative-path fmt attempt failed path lookup. The required lib.rs recursive check reported645 existing diff lines in unrelated child modules; none were reformatted. q.py could not start Bash (`Bash/Service/CreateInstance/E_ACCESSDENIED`); native rustfmt formatted only the new file, then the absolute-path qbuild fmt check passed. No lane-guard denial was received.

### Immutable real-game controls

All requests/verdicts/logs are retained under `build-runs/game-queue/done/` and `build-runs/game-results/`. Labels below are exact. Each request was unique and waited by `build-runs/cmdline4-r2-wait.py`, never qwait. Spandau's first540s wait expired; a second wait accepted the completed verdict without deleting or replacing it.

| Label | Verdict / final date | Queries | Full save / shutdown | Reference warnings |
|---|---|---:|---|---:|
| cmdline4-r2-native-2032-a | stalled /2032-01-01 |123/123 exact| save queued, no completion; no shutdown |0|
| cmdline4-r2-current40-2032-b | **clean /2032-02-01** |123/123 exact| **full save ok1,11094ms on2032-01-06; PostSystemServiceShutdown reached** |0|
| cmdline4-r2-native-2032-b | stalled /2032-01-02 |123/123 exact| save queued, no completion; no shutdown |0|
| cmdline4-r2-native-spandau-2032-c | stalled fallback /2015-03-08 |123/123 exact| no2032 save; shutdown at900s timeout, watchdog_stalls1 |0|
| cmdline4-r2-native-probeoff-2032-d | stalled during autoload /no established city date |disabled,0 comparisons| no save/shutdown or final AUTOTEST verdict |0 logged|

Spandau's final game line says clean but has watchdog_stalls1, and the immutable harness verdict is stalled; it is not counted as a pass. Probe-off-d stalled while LoadCity was running and returned no final game verdict. This demonstrates another native control failure without invoking the new queries, but does **not** isolate the two native full-save stalls or establish their cause.

- Native-a immutable DLL: `build-runs/cmdline4-r2-immutable/FreeCity4.dll`, source07aa4988, SHA256 `2D002BDDF1F1B967FC93367223930F32F5F193B484F21728FA2A58CFABA2A317`.
- All later requests used exactly `build-runs/cmdline4-r2-immutable-b/FreeCity4.dll`, source92e01745, SHA256 `053BD7A077A43A31ACB065C3B985D792CD9FEBFADAA82B0E7A343D742621FABC`, rechecked after all runs. No DLL was rebuilt during these controls.
- Current40's actual enabled set was measured as42 entries: simulator/budget defaults plus exactly the requested40, set differences0. The requested list is preserved in its request. No experimental terrain/dispatch/effects were enabled. Native requests used disable=all; native-a's ineffective sim_turbo warning was avoided in subsequent native requests. No shadow differences occurred, so no turbo-only difference required a normal-speed replay.
- `build-runs/cmdline4-r2-audit.py` reproduces the five-run accounting from retained logs. The script used for later runs is `build-runs/cmdline4-r2-save-b.script` (save2032-01-02, end2032-02-01). No UI actions, game-file edits, or unsupported reload were attempted. The documented queue resets each city and provides no export of these newly saved slot files for a supported reload; reload was not measured.

### Questions and next steps

No new spec questions or parser corrections: every observed query matched. Prior CL4 in `docs/specs/questions/command-line.md` remains open alongside CL1-CL3. No spec/ABI table, shared tool, default, game executable, or analysis-source edit was made.

The unresolved requirement is a **clean native full-save/shutdown control through2032**. Establish a supported native baseline that completes loading and full save, then compare probe-off/probe-on with fresh unique labels and the retained immutable-b DLL. Do not weaken watchdog thresholds or claim the current40 pass substitutes for native success. Further private owned construction/mutation/copy/lifetime work still requires CL4's published contract and remains outside this phase. The merge record is wip, with no owner/default promotion.

## Prior round report (retained)

The prerequisite audit stopped before 15 tool calls. No fully documented owned original construction path was established. Filed CL4 in `docs/specs/questions/command-line.md`; production core, CAPI, plugin, defaults and ABI tables are unchanged. The reserved worktree was reused with detached HEAD and an initially clean tree.

### Published facts and precise gap

- `docs/specs/command-line.md` sections 1.1-1.2 provide size `0x3C`, parsing constructor `0x00914E14`, copy constructor `0x00914A30`, nondeleting destructor `0x00419490`, reference count zero at construction, and deleting-destructor/free behavior.
- Section 3 and `docs/abi/vtables.tsv` provide the 17-slot original interface; CL1's slot-16 ambiguity remains open. `docs/specs/framework-rz-files.md` sections 8.1-8.3 repeat layout/constructor behavior without supplying the missing construction binding.
- `docs/abi/allocators.md` documents the global heap allocate/free signatures. This is positive allocator evidence, not proof that arbitrary plugin-owned storage and a guessed constructor prototype are supported.
- `docs/abi/direct-calls.md` documents a default thiscall convention for its listed functions, but contains no command-line constructor/copy/nondeleting-destructor entry establishing their exact call ABI, storage alignment/initialization, or supported destruction sequence.
- The public SDK supplies `cIGZFrameWork::CommandLine()` for the application's process command line and the abstract `cIGZCmdLine` interface. No documented owned parse/copy factory or creation CLSID was found in the audited specs/ABI/SDK declarations. The SDK's switch-overload order differs from the original slot table; it was not used as an original dispatch binding.

### Measurements and retained limitations

- Constructed original comparisons: **0/0, not exercised**. Natural original comparisons: **0/0, not exercised**. No exactness or coverage credit.
- Acquisition attempts refused before execution: direct constructor/copy calls without the complete typed/storage contract; mutating the borrowed process object; inventing a factory/private allocator; substituting an SDK implementation or a second oracle. No runtime acquisition failure is claimed.
- Native differences observed: none, because no native calls were made. Fallbacks/resynchronizations: zero. Original references acquired/released: zero/zero; in-game reference warnings were not measured.
- Game requests: none outstanding or completed. No 2032 own/native/current40 controls, save/shutdown or reload ran; prerequisites were unavailable. No game files were accessed.
- Cargo/clippy/ctest/fmt: not run; this is a question/report-only change. The task-specific instruction forbids spending the unchanged baseline/build budget when binding is unavailable. No engine implementation or exports changed.
- CL1 slot 16, CL2 empty switch name, and CL3 high-byte/locale questions remain unchanged. No speculative core correction was made.
- The generated questions index is intentionally untouched: this lane may edit only its question file under `docs/specs/` and may not run `tools/` scripts. The coordinator can regenerate the index through its normal permitted workflow.

### Next steps

Publish CL4's exact construction/copy/nondeleting-destruction ABI and storage/lifetime contract, or a supported original owned factory/parse/copy service. Then compare actual production core/CAPI with independently supplied ASCII ordinary/quoted/slash/colon fixtures and safe insert/erase/join/copy/lookup operations, retain every difference/refusal, run focused checks and one final full gate, and perform uniquely labelled 2032 controls with save/shutdown. Lane remains blocked; any merge-queue record is `wip` and carries no verification claim.

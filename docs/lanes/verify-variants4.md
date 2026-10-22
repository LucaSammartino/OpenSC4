| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-variants4 (bounded native variants) | `verify-variants4` / detached engine lane | in progress round3: retained round1/2 evidence below; tracing production native count mapping; baseline all started | Correct production raw count, test and reproduce350 comparisons, exact house40 ON/OFF save/shutdown/reload |

## Handoff
- Goal: final round3; published scalar GetCount=0, array length; preserve logical count consumers.
- Done: retained HEAD32bbaa27; read data-formats5.2/9 text/10.5 item42 and foundations4.1 exact count rules.
- Baseline all PASS: cargo4947/0, clippy0, ctest68/68; logs copied to build-runs/evidence-r3-baseline.
- Implementation: logical Variant::count unchanged; native_count/raw_count scalar0/arraylen; new CAPI export fc4_variant_get_count; production cFC4Variant GetCount uses installed Rust hook. ABI exports changed (no version/table edit).
- Verifier compares original vs independently seeded actual production objects, plus literal Rust byte snapshots; all350 observations retained. Count hook absence refuses; all mismatches logged, fallback/resync explicit.
- Focused quick PASS: cargo3453/0 (core2925+capi528), dependents compile, C++140/140; tree changed during quick, final stable all required. Both new count tests passed; absolute-path fmt PASS2.
- Done1ee5d9b6: production raw-count mapping and comparisons committed. Stable all PASS at this code: cargo4949/0, clippy0, CMake2/2, ctest68/68(84.8s), no tree-changed warning; fmt2 PASS.
- Immutable DLL/PDB: build-runs/dlls/vv4-r3-production/FreeCity4.dll/.pdb; DLL SHA256 D50E20D91AFB458388A11A03910849B22C439837C149A560753791F4D5016D28.
- Outstanding: vv4-r3-house40-on-1ee5d9-1930, exact house40 equality validated40keys before atomic submit, variant_shadow1/refaudit1/2032/full-save script. Wait helper running; do not resubmit or discard completed verdict.
- Next: inspect ON result/counts/save/shutdown, preserve saved .sc4 via supported fixture_city route, then OFF and supported ON/OFF reloads. Request/wait helper build-runs/vv4-r3-run.py checks exact list and uses no qwait.
- Exact house40 from task excludes terrain/dispatch/effects and includes all default keys; validate equality before submission.
- Original profile cap exhausted3/3; use corrected house40 only. Unique labels, preserve completed verdict, no qwait.
- Prior scalar mismatches28 and opt-in profile crashes remain evidence; HG4-HG6 unresolved. HG7 already published.
- Detached HEAD; no shared tools/maps/defaults/ABI tables, no terrain edits, no subagents, no restart/merge/push.
- Main q.py Bash transport denied access; use capped PowerShell local commands.

## Round2 report: terrain/view integration blocker

The same-request train97 control reproduces the current40 failure. Disabling only the candidate's terrain replacement reaches2032 clean/ref0, performs the full scripted save, and completes shutdown. This isolates the failure to the terrain replacement boundary on this fixture; it does not establish a corrected current40 candidate. No engine code was changed in round2: a small pointer-only fix cannot satisfy the remaining published initialization and layout requirements. Stop here for a complete terrain compatibility implementation, without disabling coverage or changing defaults.

### Controlled evidence

All requests use Konradshohe, turbo1, end_date2032-01-01, seconds900/timeout1050, refaudit1, and the identical immutable three-action build-runs/variants-save.txt. They use the ordinary harness-mirrored region alias, not fixture_city; no independently hashed city source is claimed. Script SHA256:B421EB88ACC9BED7EDBD737527B1AFDF097FEDA7D33FD9E41998792F5F58F93E. Each label was unique, each wait preserved existing verdicts, and no request remains outstanding.

- vv4-r2-train97-off-1853: corrupt/ref0 before date. Relative to round1 OFF, only DLL path and unique label changed. Preserved baseline SHA256:8A7160CAD839F19B8FD9F6D685926A6D07DEFA11D116B0FF97589A302F17A0C7, verified against build-runs/evidence-run4/run4-train97-dll.json. Its gated68/PASS joint train97-span565-joint-20261022a is a different setup; baseline success does not transfer to this request.
- vv4-r2-no-citydetail-1858: corrupt/ref0 before date. Relative to round1 OFF, only disable=citydetail and unique label changed. This excludes citydetail removal alone as a remedy.
- vv4-r2-no-terrain-1901: clean2032-01-01/ref0,145.844s,14.85 days/s. Relative to round1 OFF, only disable=terrain and unique label changed. SaveCity(full) returned ok=1 in11094ms; all three script actions completed; PostSystemServiceShutdown reached. This is a diagnostic exclusion, never full current40 acceptance.

Candidate evidence remains build-runs/dlls/vv4-e78179/FreeCity4.dll/.pdb, DLL SHA256:A7BAE0DBBAF83CA32C22B836704F4D050C1D1C2615E23235E29C0453382E0EFF. Original round1 ON/OFF failures remain valid and unchanged. No reload acceptance was attempted after the integration blocker was isolated.

### Owned symbols and published-contract findings

The matching candidate PDB resolves DLL+DC6841 to SC4String::FromChar+0x51, DC6437 to its constructor+0x57,911AB5 to Attempt+0xF5,91127C to AutoloadOnTick+0x37C,9176A9 to observer OnTick+0x89. The baseline PDB resolves E0CAF1/E0C6E7/92B995/92B15C/9314D9 to identical names/displacements. These are raw stack-scan hits, not proven unwind frames. Logs show successful progression through city load and subsystem Init before the final terrain-view creation milestone. The constructor hits do not prove a string fault. The helper build-runs/owned-symbols.py loads only owned images/PDBs through DbgHelp; no native code bytes or disassembly were read.

Concrete owned gaps against terrain.md:

- Section15.5.1 and15.5.11 require construction, Init(terrain), then viewReady. cFC4Terrain.cpp:848 sets ready first;858-861 creates/stores a view without any view Init.
- Section15.5.11 item1 requires GetView return the terrain-view interface, not the object base or another interface. The adapter requests GZIID_cIGZUnknown at858 and returns that void pointer cast as cISTETerrainView at541, without QI or adjustment. A cast cannot establish COM interface identity.
- Section15.5.11 items12-14 require live directly readable terrain vertices at terrain+0x6C and water records at water+0x18. The replacement stores private HostContext/core state instead of establishing that compatibility layout. Calling Init alone is unsafe.
- Section15.5.11 item15a requires view Shutdown before releasing the terrain's reference. Current teardown releases the created view without that explicit shutdown sequence.
- Section15.5.13.5 permits dropping the saved reference and creating a fresh view; dropping it alone is not a demonstrated violation. A fresh view still needs Init. Full save/reload validation belongs in the complete fix.

The boundary is experimentally isolated; the exact native fault has not been attributed to one individual gap. Fix identity, complete initialization/layout/lifetime together, then rerun the full matrix. Returning null or removing terrain is not an acceptable fix.

### Gate and scope

Initial qbuild all passed cargo4947/0,clippy0,ctest68/68 but noted a tree change from the lane report. Final qbuild all passed the same counts with no tree-changed warning: CMake build PASS, ctest68/68 in76.35s. Logs:build-runs/qbuild-test.log,qbuild-clippy.log,fc4verify-build.log,fc4verify-ctest.log. No Rust edits, so fmt is inapplicable. q.py could not launch Bash (E_ACCESSDENIED); bounded native PowerShell was used. No lane-guard denial.

No core/count/conversion, ABI/export/table, default, coverage, or generated-file changes. Original322/350 observations and all28 HG7 scalar-count differences remain intact; HG4-HG7 remain open, no new spec question needed for explicit terrain contracts. Native/own/current40 ON+OFF2032/ref0 and supported reload acceptance are still incomplete. No readiness, owner promotion or coverage credit is claimed. Round2 milestones:5e76bba3 retained evidence;f12f9985 same-request baseline failure and owned symbols; this report records the completed exclusion control and stable gate. Detached HEAD only; no merge/rebase/ref update/push or subagent.

## Retained round1 report

The bounded native verifier is implemented and the stable full build gate passes. The lane remains WIP because the current40 integration controls fail before the first simulation date with the shadow both enabled and disabled. No whole-owner equivalence, replacement promotion or coverage credit is claimed.

### Implementation and ownership

`engine/plugin/FC4VariantShadow.h` adds an opt-in `variant_shadow = 1` experiment through the existing formats city-load hook. The setting is additive at the end of FC4Config's list and defaults to zero. No C exports, ABI version/table, core semantics, unrelated subsystem, or generated ABI entries changed.

The route uses the SDK resource manager to own an exemplar, borrows its property and value, then calls the corrected compiled interface's CreateCopy. Borrowed objects are never mutated or released. Owned results are audited with FC4_ACQUIRED and released with FC4_RELEASE_CREATED. The source has the published foundations 17.1 identity; owned SDK clones may use a different native COM vtable. The compat layer identifies the supported image, and all 127 published interface entries are validated as executable inside that image. No clone address, private object layout or native implementation bytes are used.

There are two distinct scalar literals and arrays of lengths 0, 1 and 3 for each of the fourteen numeric/character types. Native scalar setters/value/out getters, array copying setters/data getters, clone isolation after source mutation, CopyFrom state and Erase state genuinely execute. Inputs enter Rust through fc4_props_set_typed before native getters run. Rust serialization exposes that independent typed storage; native getter results are never fed into the Rust oracle. Native copies are compared against an immutable Rust typed snapshot, not a second Rust implementation or a resynchronized state.

### Measurements and limits

Each successful experiment executes 70 native cases and creates/releases 71 owned native clones. Its 420 checks comprise 70 Rust literal-storage setup checks plus 350 native observations. Setup checks pass70/70; native observations match322/350. Thus the logged aggregate is392/420 exact, with28 failures,0 refusals and0 unexplained differences. Each type reports30 checks,28 exact,2 scalar-shape failures.

HG7 accounts for all28 differences: observed native scalar GetCount=0 versus Rust Variant::count=1. Every tested native scalar type matches its expected type. Normal speed reproduces the discrepancy, so it is not turbo-only. No expected count or Rust behavior was changed to hide it. Typed scalar values, arrays/counts, clone isolation, CopyFrom contents and Erase metadata match in these bounded cases.

This is not equivalence of all127 methods or all inputs. Cross-type conversions, As/reference aliasing, string operations, objects/pointers, raw-void arrays, float exceptional cases, native segment serialization/read-into, and a dedicated Rust Clone-operation comparison are not established. HG4 requests a published isolated ORIGINAL segment-stream fixture; serializing native getter output with Rust would not verify native serialization. HG5 requests the missing conversion/lifetime contracts. HG6 requests definitive Erase/CopyFrom/CreateCopy return channels and clone identity; their nominal bool results are discarded and success is judged through output/state.

### Gates and real game

Final stable `qbuild all` at code commit e7817958: cargo4947 passed/0 failed, clippy0 diagnostics, CMake build PASS, ctest68/68. It completed without a tree-changed warning. Earlier return-channel review failures were corrected, and quick gates passed after each code milestone. No Rust file was edited, so rustfmt checks are inapplicable. FC4_CHECKED is OFF: these are support-image/interface guards, not an RTC build.

| Unique request label | Result | Variant evidence and save |
|---|---|---|
| vv4-original-a567f1-1820 | clean2032-01-02/ref0 | cases0/refusal1: initial strict clone-table identity refused the distinct native COM implementation; full save ok=1 |
| vv4-original2-7c5ca4-1810 | clean2032-01-04/ref0, turbo | cases70/checks420/exact392/HG7 failures28/refusals0/clones71; full save ok=1 |
| vv4-normal3-e78179-1815 | clean2032-01-01/ref0, normal | same70/420/392/28HG7, unexplained0; full save ok=1;125.484s,17.26 days/s |
| vv4-current40-e78179-1819 | CORRUPT/ref0, no date | experiment completed with same70/420/392/28HG7/unexplained0 before foreign access violation at0x41800000; no save reached |
| vv4-current40-off-e78179-1824 | CORRUPT/ref0, no date | variant_shadow=0; no variant experiment executed; same foreign access violation at0x41800000; no save reached |

The original profile cap is exhausted3/3; the last two requests are current40 integration controls. Each label was unique, only one request was outstanding, existing completed verdicts were preserved, and all requests are finished. No further native trial or unrelated subsystem fix was attempted. Full saves succeeded on all three original profiles. Edited-save reload and native variant persistence were not validated: the owned experiment is ephemeral and no isolated native segment-stream fixture is published (HG4).

The exact current40 enable list used for both controls was: landvalue,weather,ordinance,demand,power,plumbing,flammability,pollution,protection,crime,aura,flora,landfill,residential,zones,neighbors,development,traffic,tract,lotdev,commercial,industrial,civic,parks,history,clock24,dispatch,citysit,demolition,propdev,citydetail,ports,effects,nettool,terrain,netmgr,netlot,propmgr,lotcfg,occmgr. The failure also occurs with the experiment disabled; its underlying cause is not diagnosed here.

Final immutable read-only DLL/PDB: build-runs/dlls/vv4-e78179/FreeCity4.dll and FreeCity4.pdb. DLL SHA256 before and after controls: A7BAE0DBBAF83CA32C22B836704F4D050C1D1C2615E23235E29C0453382E0EFF. Detailed verdicts/logs and scripts remain in this worktree's build-runs. No game files are committed.

### Questions and next steps

HG4-HG7 are appended to docs/specs/questions/foundations.md; the generated index was not edited. Resolve the failing current40 control through the coordinator's integration work before readiness. Publish the scalar-count and remaining behavioral contracts before expanding verification or changing core semantics. Native serialization/reload remain unverified. No requests are outstanding.

Milestones: a567f1d5 bounded implementation;7c5ca461 owned-interface/return-channel correction;e7817958 final diagnostics;98247d05 stable gate and normal evidence;f960ee4a current40 failure record. Detached HEAD commits are for supervisor publication to the reserved branch; no branch/ref/config changes, merge or push occurred.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-variants4 (bounded native variants) | `verify-variants4` / detached engine lane | ready round3: production Rust/CAPI GetCount scalar0/arraylen; native350/350 twice, prior28 differences retained; exact house40 ON/OFF and supported reload ON/OFF clean2032/ref0/save/shutdown; stable cargo4949/0 clippy0 ctest68/68; ABI export added; HG7 published, HG4-HG6 remain | HG4-HG6 contracts required before broader conversion/native serialization/return-channel claims |

## Final round3 report

Ready for the bounded native-count and verifier change, implemented in `1ee5d9b6`. Both exact-house40 controls and both supported saved-city reloads succeed. The earlier 28 scalar differences and all older failed runs remain below as historical evidence. No fourth original profile was run; the original-profile cap remains3/3.

### Published rule and production mapping

HG7 was already answered: data-formats §5.2 (line842) says scalar GetCount=0 and array count=element length; lines1916-1918 give fresh SetValUint32 count0; §10.5 item42 (line2107) distinguishes scalar7 count0 from array-one count1. Foundations §4.1 (line545) independently states scalar variant count0. The question file now appends this correction without erasing the previous measurements or inventing a new native contract. No new question ID was needed.

The owned native interface is `cFC4Variant`, whose storage and typed getters use the SDK. Its old GetCount forwarded the SDK's stored raw count; the verifier incorrectly compared native scalar metadata to logical Rust count1. The production adapter now invokes `fc4_variant_get_count` through the installed property bridge, which calls core `variant::native_count`. `Variant::raw_count` shares that mapping. Existing `Variant::count` keeps logical scalar/object count1 and array length; its misleading GetCount documentation is corrected. Typed setters, scalar byte extraction, converting getter input counts and serialization still use their existing logical lengths. No consumers were blindly changed.

The standalone plugin-load test has no linked Rust library and retains an explicit spec-shaped fallback. The real-game experiment refuses to proceed unless the hook equals the actual production CAPI function, so fallback cannot earn comparison credit. This adds one C ABI export, `fc4_variant_get_count`; ABI version and tables remain unchanged. No tools/maps/defaults/generated ABI entries or terrain code changed.

### Actual original/production comparisons

The same fourteen numeric/character types each execute two distinct scalar literals and arrays of lengths0,1,3:70 cases. Every original typed setter/getter, scalar out getter, array copying/data getter, clone isolation, CopyFrom state and Erase observation is retained. The verifier independently seeds actual `cFC4Variant` objects with those literal inputs, calls their real virtual getters and copies, and compares them with the owned ORIGINAL results. It also retains the independent Rust typed-storage byte snapshots, seeded before original getters. No native result seeds Rust, no private verification implementation supplies count metadata, and no mismatch is filtered or resynchronized.

Each ON run measures native350/350 plus Rust literal setup70/70, aggregate420/420. All28 former scalar-shape failures now compare against the actual production Rust/CAPI getter and pass; each type reports30/30. Per run, actual production calls are core native_count196, CAPI typed-set70 and variant-write70; native owned clones71 and production clones70. Failures0, scalar-shape differences0, unexplained0, refusals0, production-clone refusals0, fallback0, resync0. The count counter wraps real production virtual invocations with the Rust hook verified; typed values and copying remain backed by the production SDK, not a claim that every variant method runs in Rust. All mismatches are logged without the old first12 cap. OFF runs contain zero variant experiment lines.

### Gates

Baseline all at retained `32bbaa27`: cargo4947/0, clippy0, ctest68/68; baseline logs preserved in `build-runs/evidence-r3-baseline`. Focused quick: cargo3453/0 (core2925/capi528), dependents compiled, C++140/140; it reported a changing tree during implementation and is not the final stable gate. Both discriminating core/CAPI tests passed (scalar7, empty/one/multi arrays, stale scalar storage metadata), and absolute-path rustfmt checks passed for both changed Rust files.

Stable final `qbuild all` at `1ee5d9b6`: cargo4949 passed/0 failed/0 ignored, clippy0 diagnostics, CMake2/2, ctest68/68 in84.8s, no tree-changed warning. No engine source changed afterward. Full logs remain `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. FC4_CHECKED remains OFF.

### Exact house40 and immutable game evidence

All four requests were compared byte-for-byte with the supplied canonical enable CSV before atomic submission and rechecked from their archived requests after completion. None contains a disable override. The exact list is:

`landvalue,weather,ordinance,demand,power,plumbing,flammability,pollution,protection,crime,aura,flora,landfill,residential,neighbors,zones,development,tract,history,clock24,commercial,industrial,civic,parks,lotdev,zonedev,traffic,automata,netmgr,netlot,nettool,demolition,occmgr,propmgr,lotcfg,citysit,propdev,simgrids,citydetail,ports`

All use Konradshohe, turbo1, end_date2032-01-01, seconds900/timeout1050, refaudit1 and the unchanged three-action `build-runs/variants-save.txt`. Script SHA256:B421EB88ACC9BED7EDBD737527B1AFDF097FEDA7D33FD9E41998792F5F58F93E. Every run finishes all3 script actions, SaveCity(full) ok1, and reaches PostSystemServiceShutdown. All verdicts are clean2032-01-01/ref_warnings0, with no foreign exception, watchdog stall, RTC or overrun.

| Unique request label | Shadow and comparisons | Real seconds / days per second | Full save ms |
|---|---|---|---|
| vv4-r3-house40-on-1ee5d9-1930 | ON native350/350, setup70/70 | 189.703 /11.41 | 11156 |
| vv4-r3-house40-off-1ee5d9-1935 | OFF zero experiment lines | 174.750 /12.39 | 11265 |
| vv4-r3-house40-reload-on-1ee5d9-1938 | ON supported reload, native350/350, setup70/70 | 122.750 /11.91 | 11047 |
| vv4-r3-house40-reload-off-1ee5d9-1941 | OFF supported reload, zero experiment lines | 103.859 /14.07 | 11172 |

Immutable DLL/PDB: `build-runs/dlls/vv4-r3-production/FreeCity4.dll` and `FreeCity4.pdb`. DLL SHA256:D50E20D91AFB458388A11A03910849B22C439837C149A560753791F4D5016D28, unchanged through all4 runs. After the ON save, its logged slot1 `.sc4` (mtime19:31:25,1003482bytes) was preserved as main `build-runs/fixtures/vv4-r3-house40-on-1ee5d9-2028.sc4`. Both reloads use the documented `[ini] fixture_city` route; the harness confirms SHA256:9C181832B9DAC038D22BF90838E5A0105CEA65F1CC15CE19EF2AD2A72516EE90 and source preservation. That hash remains unchanged after both reloads. No game file is committed. Original ON/OFF runs use the mirrored alias and make no fixture-source hash claim.

Requests, verdicts, logs and per-request profile JSON are in this worktree's `build-runs/game-queue/done`, `game-results` and `build-runs/<label>-profile.json`. The local submit/wait helper is `build-runs/vv4-r3-run.py`; no qwait, external workflow, game restart, new original profile, terrain edit or coverage reduction was used. Each label is unique, completed verdicts are preserved, at most one request was outstanding, and none remains outstanding.

### Limits and retained failures

HG4-HG6 remain unresolved in `docs/specs/questions/foundations.md`: native segment serialization/read-into fixture, conversion/lifetime contracts, and Erase/CopyFrom/CreateCopy return channels/identity. Nominal native bool returns remain discarded where their channel is ambiguous; state/output establishes only the bounded observations. These runs do not establish all127 methods, exhaustive equivalence, untested conversions, strings/objects/pointers/reference aliasing, exceptional floats, native variant persistence or an entire Rust variant owner. Supported house40 save/reload is established; the ephemeral variant experiment itself is not persisted. No current turbo discrepancy remains to trigger another normal-speed run; the old normal-speed28-difference run remains recorded below.

The older list called current40 included terrain/dispatch/effects and omitted automata/zonedev/simgrids. Its ON/OFF crashes, train97 crash, no-citydetail crash and minus-terrain diagnostic success remain valid for that opt-in profile. They are not failures of actual house40. The retained historical reports below use their original profile names and conclusions; this final report supersedes that profile interpretation and HG7's former unresolved status. Next work beyond this lane requires published HG4-HG6 contracts, not a whole-owner promotion or a terrain workaround.

## Retained round2 report: opt-in terrain/view integration blocker

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

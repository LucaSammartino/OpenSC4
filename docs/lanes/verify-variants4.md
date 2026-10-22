| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-variants4 (bounded native variants) | `verify-variants4` / detached engine lane | in progress: round2; retained code e7817958; native322/350,28 HG7; current40 ON/OFF corrupt/ref0 | (1) Resolve owned crash frames. (2) Run preserved train97 identical control. (3) Fix only evidenced owned cause and validate. |

## Handoff

- Goal: diagnose current40 ON/OFF foreign exception using preserved train97 gated68/PASS baseline, no RE or core semantic changes.
- Retained report follows; original report commit6e1f9257, copy build-runs/round1-report.md.
- Baseline evidence: build-runs/evidence-run4/train97-gated-8a7160ca.dll/.pdb; SHA8a7160cad839f19b8fd9f6d685926a6d07defa11d116b0ff97589a302f17a0c7.
- OFF owned frames: DLL+DC6841,DC6437,911AB5,91127C,9176A9; resolve with retained PDB next.
- Same fixture is harness Konradshohe alias; script build-runs/variants-save.txt; exact request in game-queue/done.
- Baseline qbuild all running exec session97202; no game request outstanding.
- q.py Bash launch E_ACCESSDENIED; bounded PowerShell used. No guard denial.

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

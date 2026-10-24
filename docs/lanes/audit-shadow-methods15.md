| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| audit-shadow-methods15 (method-level credit for seven partial shadows) | `Codex` / `audit-shadow-methods15` (RE docs lane, detached HEAD) | ready: attribution `072d3d9ee`, 44 functions / 4070 bytes; viewutil 1/78, querytool 2/147, legacyctl 4/96, gdriver 12/1706, renderprops 9/1064, app 15/886, modelinst 1/93 (functions/bytes); consistency PASS, doccheck PASS (resumed round confirmed); existing scoped runs exact; ABI unchanged; no questions filed | (1) Coordinator consumes method rows, handles duplicate/folded names and lands docs; (2) retain the documented exclusions. |

## Report

Proposed [method attribution](../specs/attribution/proposed-shadow-methods.tsv) credits **44 distinct named function bodies, 4070 bytes**, using measured comparisons on game data. This is a proposal for the coordinator's method-key change, not a coverage-map edit or whole-owner verification. Byte counts are complete exported function-body sizes, not counts of compared fields or observed memory bytes. In particular, modelinst observes only three destination bytes of its named body.

| Shadow | Proposed functions | Function bytes | Exact measured family |
|---|---:|---:|---|
| viewutil_shadow | 1 | 78 | rotation 73/73; final viewutil-subset 73/73 |
| querytool_shadow | 2 | 147 | querytool 73/73 |
| legacyctl_shadow | 4 | 96 | legacyctl 72/72 |
| gdriver_shadow | 12 | 1706 | gdriver 72/72; 35 transition samples |
| renderprops_shadow | 9 | 1064 | renderprops 73/73; live-owner registry subset |
| app_shadow | 15 | 886 | app 72/72 and app-startup 1/1 |
| modelinst_shadow | 1 | 93 | modelinst-live 72/72; 4096 captured calls |
| Total | 44 | 4070 | Existing runs only; no new game run |

### Frozen evidence

TSV citations use the first nine characters of these commits. Source line numbers and original run-record numbers refer to these snapshots; integration may renumber the run files. Each row cites both a comparison site and its measured run record. The corresponding lane report supplies per-family/per-slot counts when its run record has only an aggregate.

| Branch read | Full snapshot commit |
|---|---|
| glue-viewutil-apply12 | `98263d002530af79268ea186e528cc38f4057749` |
| glue-querytool-shadow13 | `36532459b4f98fbfbe30d34cbb7b066eed3a48ea` |
| glue-legacyctl-shadow12 | `ab3654c1c6e75fdf5ad22eccc3541ef694348f00` |
| re-legacyctl-lc16 | `45ac2f807488cfd62d59d338003873330726a833` |
| glue-gdriver-shadow13 | `0bb966d71789104787b9d496185de551ac6bbbc1` |
| glue-renderprops-shadow13 | `1a75bc97b7f2080f60194a8262a91e2048e8db85` |
| glue-app-shadow13 | `10d0d3c0d7a99d0f4b707aa380af4829e970406e` |
| glue-modelinst-shadow12 | `25e148fcdf7722d1ffabbaa69a950a6acb3f5f93` |

Also read the earlier `glue-viewutil-shadow11` report. Its run `viewutil11-r1-on2-20261024-1424` measured only rotation, 73/73. Its VU16 restriction is superseded by the VU15/VU16 contracts used in apply12; that extension still supplies no eligible game-data comparison for the excluded fixture families.

### Scope decisions and exclusions

- **Viewutil:** retain only the free rotation helper's comparisons using five independently copied live camera vectors and the live rotation (`FC4ViewUtilShadow.cpp:293`, `:357`, `:359`, comparison `:331`). This has no fabricated owner/iterator receiver; the copied vector is the actual game input to a pure helper. Fixed-vector calls add no credit. All 17 local families use constructed packets (`:209`); all 18 cached families use a private copied owner and generated coordinates (`:125`, `:129`, `:134`). Exclude these, including the 32 other exact subset families. Object-angle, camera-angles and city-width also differ in every observation; VU12/VU13/VU17/VU18/VU19 and the 14 unmeasured bodies remain outside the proposal. No camera/frustum construction credit follows from reading its inputs.
- **Querytool:** original filter calls consume borrowed live occupants; type/property-presence inputs independently feed the core predicates. The two eligible owners are the hover and describe filter classes, not `cSC4ViewInputControlQuery`. Neither filter construction nor occupant getters are independently compared. CT27 blocks the controller's renderer-hit, target, displayed-field/string and highlight behavior.
- **Legacyctl:** retain QueryInterface, AddRef, SetOwnControl's body, and GetControl. All five owner fields plus relevant returns/output identity are compared from pre-call state. The exact run records qi=2367, addref=3270, own=775, control=1564, builds=775 and build_getters=775. SetParameters only counts calls and triggers a separate GetControl probe: no SetParameters result/property-interpreter replay credit. LC14/LC15 were answered by re-legacyctl-lc16 but the shadow still lacks those comparisons. The two ownership hook paths share one counter, so the 8-byte adjustor `0094c4d8` has no separately established execution count and is conservatively excluded; either path reaches the credited body `0094c314` (adjustor tail at `0094c4db`). No Release/destructor or other interpreter credit.
- **Gdriver:** retain eleven exercised hook entries: slots 12/13/28/40/41/44/45/51/62/74/75, with sample counts 1/1/4/4/4/4/4/4/4/1/4. Also retain original slot76 GetViewport, called and compared after each of the five sampled viewport transitions (`FC4GDriverShadow.cpp:178`, `:181`; core `shadow.rs:144`, `:170`). Slot43 only advances the frame counter. Exclude slots 14/26/46/48/54-57 because the lane explicitly reports their hook paths unexercised. Slot46 can serve as a capability readback (`:83`), but the aggregate does not say whether those samples took that branch; it gains no additional credit. Pre-state is reset per transition. Device transform/render-state effects, dirty/applied caches, texture formats and lifecycle remain unverified.
- **Renderprops:** the borrowed application's live render-properties object reaches the shared registry comparator (`FC4RenderPropsShadow.cpp:35-36`). Its nine entries are three counts, three names and three lookups, totaling 542 independently predicted registry fields per observation. The 41,398 other fields, including setters/value getters, owned registry stability, constructor, refcount and termination checks, are isolated fixtures and excluded. RP7's required complete input/copy history and lifetime-safe complete-view observation remain unavailable; live configured settings, preferences/hardware replay and scroll snapshot gain no credit. RP8 does not affect the retained borrowed-owner registry calls. Scope is C-locale ASCII probes only.
- **App:** retain 15 named entries among the 17 supported fields. Stored paths are captured once from fields, lazy folders/filenames are independently modeled, and the native mode getter is compared to message-derived state. Exclude AP18 slots65/66 (the normal-speed diagnostic also differs), every AP17 counter/pause/speed/startup/idle/pump path, and the input-only GetAppName call. Slots57 (`00449e60`, 21 bytes) and60 (`00449f00`, 84 bytes) are measured but coverage's name resolver returns no name; leave their 105 bytes uncredited pending naming. Slot64 is supported; it must not be confused with AP18 slot66.
- **Modelinst:** only the live static GetTransform destination counter and two padding bytes qualify. The first live line has 1426 calls/4278 fields; final captured=4096, encountered=93481, dropped=89385. Fixture calls are explicitly excluded from those counters. Slot9's unnamed 8-byte adjustor `00802750` tails at `00802753` to the named body `00801b80`; retain only that 93-byte body. Its other output words/kind, constructors, transforms/bounds on constructed instances, draw attributes, model/material state, animated/basic owners and resources gain no credit. This is observed-effect coverage, not full transform-output verification.

### Coverage identity and validation

Ran `python C:/Users/tlori/FreeCity4/tools/coverage.py --owners build-runs/audit-owners.tsv --out build-runs/audit-coverage.md`: PASS, game code 29,535 functions / 5,584,344 bytes. Resolved names with the actual `Attribution.name` method (committed names first, nondefault export names second), then joined owner and body bytes by exact address. Local outputs remain untracked. PE table words resolve interface dispatch where curated ABI tables lack slots; export queries precede the byte-verified adjustor checks. No REA session was required.

Coverage names/owners are retained literally even when folded: legacy AddRef `0094c47c` is `cGZWin::AddRef` under `cGZWin`; legacy GetControl `009c32ac` is `icf::cGZApp::ModuleName` under `<shared>`; app GetGameMode `0080bc00` is `cSC4TrafficSimulator::GetCommercialTrafficMap (folded getter [ecx+38h])` under `cSC4TrafficSimulator`. These are address identities, not evidence about those owners' other behavior. App slot54/58 labels also remain coverage's names rather than new semantic renames.

Both driver viewport setters have the exact name `nSGLDX7::cGDriver::SetViewport` at different addresses (`00883760`, `008837e0`). Both qualify here, but a method-key implementation must detect such collisions rather than assuming a name uniquely resolves one address. Do not silently promote class aliases, shared bodies, or unresolved `FUN_` names to owner-wide credit. The seven `TOTAL` TSV records summarize eligible data rows and are not functions.

Attribution consistency validation passed: 44 unique addresses / 4070 bytes, all names/owners/sizes equal to coverage, all seven totals consistent, 28 pinned evidence locations resolving. The first attempt stopped on the helper's multi-location citation parser; correcting that untracked parser yielded the PASS without an attribution-data change. `python -X utf8 tools/doccheck.py check` passed (`doccheck check: ok`); `git diff --check` passed. Logs are `build-runs/attribution-validation.log`, `coverage-console.log`, and `doccheck-audit-shadow-methods15.log`. Cargo, clippy, native build and CTest are not applicable to this docs-only audit. No engine or tools files, coverage map, ABI rows/exports/version, run records or question files were changed. Questions filed: none. The q.py wrapper failed before running coverage (Git Bash CreateFileMapping denied); the direct PowerShell coverage invocation completed successfully and is the only counted coverage run.

Resumed verification on 2026-10-24 after report commit `a399caef9`: reran coverage owners against the current main-checkout tooling, then checked all 44 addresses, literal names, owners and byte sizes, seven totals and 28 pinned evidence locations. All passed without an attribution-data change; doccheck and diff check also passed. Foreground logs: `build-runs/coverage-resume.log`, `build-runs/attribution-validation-resume.log`, `build-runs/doccheck-audit-shadow-methods15-resume.log`, and `build-runs/diff-check-resume.log`. The exact game-run evidence remains the frozen records above; no new game run was needed.

Next for the coordinator: consume only the 44 function records, implement collision-aware method matching, retain literal address/name/owner identities, and preserve every exclusion above. No build or game request is outstanding. The reserved lane ref is published by the supervisor; this worktree commits on detached HEAD and does not update shared refs.

## Handoff

- Goal: propose exact original-function coverage credit for viewutil, querytool, legacyctl, gdriver, renderprops, app and modelinst shadows.
- Done: initialization `61dee6871`; attribution and full report `072d3d9ee`; audited all seven source/adapter/core paths and named lane reports; 44 named functions / 4070 bytes with pinned comparisons and exact run records.
- Done: final report `a399caef9`; resumed current-tooling coverage/consistency PASS (44 / 4070, seven totals, 28 evidence locations), doccheck PASS and diff check PASS; ready for the local docs merge queue.
- Next: coordinator consumes the proposal and supervisor publishes the detached tip; no further lane work or requests outstanding.
- Facts: two app getters (105 bytes), modelinst's unnamed adjustor (8 bytes), and legacy's unseparated ownership adjustor (8 bytes) are conservatively uncredited; driver SetViewport has two eligible addresses with one name.
- Outputs: `docs/specs/attribution/proposed-shadow-methods.tsv`, this report, and local merge queue JSON.
- Guard: `C:/Users/tlori/FreeCity4/build-runs/run4-audit-shadow-methods15-r2-manifest.json`; budget 150000, 150 calls between commits.

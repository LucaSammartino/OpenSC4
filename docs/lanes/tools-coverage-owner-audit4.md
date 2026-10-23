| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| tools-coverage-owner-audit4: public owner qualification audit | Codex; tools-coverage-owner-audit4 (detached reserved worktree) | in progress: independent Directory and HTML qualification; previous accounting preserved | Review all published operations against current production algorithms and assertions; withhold incomplete owners; complete own report and queue. |

## Handoff
- Goal: substantive independent whole-owner qualification for Directory and HTML; preserve prior accounting and failures.
- Done: 7bde56f5 initial handoff; 196146d6 reproduced accounting; e822a6a7 corrected whole-train accounting and substantive findings; full final report is below and is committed with this handoff.
- Done: 18-owner machine table validated, four areas, 378309 attributed bytes; 89 public source hashes; all named current SPECIFIED/unimplemented owners and references validated.
- Done: pinned107 whole normalized report matches published; default108 repeat is byte-identical; independent owner/status sums match headline; protected-path assertion passes; diff check clean before final documentation-only update.
- Next exact step: compare framework chapter 6 and paths chapter 3 against authoritative rz_files directory; then complete canonical HTML operation ledger, independent literal witnesses and classification checks. No engine execution or edits.
- Evidence: build-runs/owner-closure.{tsv,json}, closure-selections.json, finalize_audit_evidence.py, accounting-reproduction.json, audit_accounting.py, owner-accounting.json and owner-audit-*.md, all local ignored evidence.
- Facts: train100 +8437 implemented bytes/109 functions through PersistResourceManager; trains101-108 all headline deltas zero. Initial final-resolution-only zero comparison was corrected.
- Facts: Directory4385 B and HTML38218 B are prior inventory candidates, NOT accepted weights or qualification. Recheck current attribution. Explicit cap0: runtime, player-drive, LegacyControl, WWW; GZ messages1 remaining; other capacities UNKNOWN unless report says otherwise.
- No engine/tests/maps/shadows/generated reports/defaults/region/ABI edits; only own lane report tracked. No game requests or new questions.
- Guard: context150k OVER denied final git+guard command after final report edit; obeyed, no merge-queue entry and no self restart. Return state continue, not ready.

## Independent qualification: first milestone (2026-10-23)

**Directory: WITHHELD.** The prior candidate designation was a review lead, not qualification. Framework §6 describes enumeration, list wrappers, recursive *listing*, create rollback and split. Paths §3.8 additionally publishes `RemoveDirectoryA(path, recursive)` (0x0091BF93): initial OS remove; early failure for nonrecursive/error 2/3/19/not-directory; recursively remove children in enumeration order; remove files only after all children succeed; final OS remove; stop at first failure. No such production algorithm exists in `engine/core/src/rz_files/directory.rs`. Its host trait (31–44) has no error-code or file-delete boundary; `host.remove_directory` at 329 is used solely for outermost-first rollback of created prefixes. This is a missing published operation, not an unknown OS wildcard or a requirement for a shadow.

Authoritative implementation: `rz_files/directory.rs`; `paths/directory.rs` reexports it, rather than implementing another Directory. DirectoryEntry is a separate exact map owner (paths §3.10: three named bodies/94 B), not additional Directory credit. `cRZPath`, find wrappers, file deletion, strings and sort templates retain their boundaries. Borrowed entry invalidation on reread differs from independently copied name lifetime. Platform encoding/wildcards are host primitives, not automatic portable gaps. Framework's older entry-layout summary (427–428) differs from paths' later PA5 owner-pointer/name/kind description; no new native-layout proof is claimed here.

Independent literal removal witness, fixed before any implementation execution: nonempty `P\\` contains child `d` and file `f`; initial remove fails with error 145; directory existence succeeds; recursive removal of `P\\d\\` fails. Expected false; initial remove, eligibility check, child enumeration/removal; **no delete of `f`, no final parent remove**. Companion cases: initial success stops; errors 2/3/19 stop before enumeration; child success then file failure prevents final remove; complete children/files success performs final remove and returns its result. Actual: **UNRUN / algorithm absent**, never executed-native disagreement. No Rust tests or engine commands written/run.

**HTML: review in progress; known contradictions already require withholding.** Canonical HD5 resets position but preserves pushback/token/root/style; `parse.rs:9–23` does not reset position. HD6d numeric conversion uses a radix heuristic and whitespace/sign-aware saturating unsigned conversion before modular negation; `mod.rs:1202–1207` instead wraps leading decimal digits. Current SCRIPT returns SkipUntilToken and source agrees (`parse.rs:137–139`); stale historical answers are not the contract. Full-operation ledger and finite witnesses are the next step. Coverage inputs remain unchanged.

Original budgets: core-htmldocument4 state 1 consumed, at most 2 remaining; re-htmldocument4 RE 3 exhausted; re-html-core-answers4 research 1. core-rzdirectory4 state 1 consumed/original limit 3; core-paths-directories4 parent 1 + contract-fix phase 2 = 3 exhausted. Recursive-helper owner assignment/budget requires explicit review and remains UNKNOWN: Directory capacity cannot restart capped Paths.

## Result and accounting

The premise needs correction: **train100 added 8,437 implemented bytes /109 functions; train101-108 are flat in all four statuses.** Whole-train comparison finds the PersistResourceManager row changing from `core=-` to `core/src/persistence_manager`. Comparing only train100's final conflict-resolution commit had missed its earlier change; that initial audit result is superseded.

The tool correctly sums its current map. Mapping lag and genuine incomplete owners both contribute to selection blindness; **no arithmetic accounting defect was proved**. Complete portable-owner claims exist for Directory and HTML while their core rows remain empty. Larger bounded implementations retain actual missing operations. Already implemented owners receiving native probes cannot add implemented bytes again.

Read [CONTRIBUTING](../../CONTRIBUTING.md), [policy](../COVERAGE.md), [tool](../../tools/coverage.py), its map/shadow inputs, published report/specs and the cited landed reports. No original binary/decompilation/disassembly/RE working material was opened for research. The existing coverage tool mechanically consumed its normal analysis inputs; only aggregate accounting and hash-only provenance were inspected. No engine/tests, coverage inputs, qualifiers, generated report, attribution, threshold, default, ABI or region change; no external communication, agents or game requests.

Independent summation of the generated per-function accounting, grouped by owner and first-match map row, exactly reproduced these integers. Attribution itself was reused from the sanctioned tool, not independently reconstructed from original code.

| Status | Bytes | Functions |
|---|---:|---:|
| denominator | 5,584,842 | 29,536 |
| specified | 3,226,906 | 14,602 |
| implemented | 2,018,463 | 8,982 |
| verified | 1,099,943 | 4,325 |
| replaced | 1,012,661 | 3,899 |

Whole-train endpoints: train100 `add06014` (+8437 implemented); 101 `ab04822e`,102 `54feae7d`,103 `bd75fb53`,104 `b5c2d06c`,105 `a0723f48`,106 `9393b6ee`,107 `4ab2aae2`,108 `640ba764` (all zero). `build-runs/accounting-reproduction.json` records full commits, preceding train boundaries, exact before/after byte/function totals, map hashes and landed report paths.

Pinned107 reproduces the **entire normalized published report**. Current default108 differs in evidence, not headline totals: terrain108/108 becomes90/90; network-manager resynchronized lines21 becomes15, while72/73 remains unverified. Refreshing that stale report alone cannot move coverage. Neither differing sets nor resynchronized outcomes were promoted.

Full input SHA-256s are in `build-runs/accounting-reproduction.json`: every report-listed input, export summary, tool, shadow evidence source, named spec and core-path existence observation. Text uses policy decoding/CRLF normalization; executable hash verification is delegated to the existing tool. Selected public source/spec/report hashes are in `owner-closure.json` (89 files).

| Input | Exact normalized SHA-256 |
|---|---|
| coverage-map.tsv | `e3c1777e6632d1f4dee70932c36855544e663587a09be481c6aaf9364b3e3981` |
| coverage-shadows.tsv | `9731b2ec2a2606296384a8c37341b8fce52dbab784e2bff84ad24c9320ad5fd0` |
| regress-train107.out | `4cc43d8f4b30a8421386cdc847f9d62788e7ae7f943eba9ab4eadc3f212028e7` |
| regress-train108.out | `52bafe4b56fecc3d6f0ab676f51edc3c02d248166e9be04d899c5d8f42848e7f` |

## Published completion obligations

`implemented` checks the paths explicitly listed in a map row; it does not discover modules. Native glue, exact shadow and replacement are separate statuses, **not implemented prerequisites**. Conversely a new path cannot honestly qualify a partial owner merely because it exists. Subject-level spec coverage, complete owner algorithms, legitimate external primitives, other owners and explicit safe-domain choices must be distinguished.

BudgetWindow's own UpdateValues result is not a primitive input; OS directory enumeration is primitive if Directory owns traversal/filter/rollback. Ambience layers are separate from their manager. ScriptServer's foreign VM boundary does not implement LuaThread or the collector. A copied native inventory is not a current attributed-byte denominator.

## Owner closure selection

Machine table: **`build-runs/owner-closure.tsv` and `owner-closure.json`**, generated/validated from `closure-selections.json` by `finalize_audit_evidence.py`. Eighteen unique SPECIFIED/unimplemented owners total **378,309 whole-owner bytes** across four areas, including the top three in each. Bytes are upper bounds, not completed subtotals or promised gains. The machine table includes exact map fields, paths, missing operations, uncertainties, budget evidence and next work.

Paths below are under `engine/core/src/`. UNKNOWN capacity authorizes no restart. First candidate means no dedicated core found in the scoped public tree scan; it does not grant three new rounds. Existing report claims are evidence for review, not fresh native measurements.

| Owner; bytes/functions | Current implementation | Missing published operations / next clean-room closure work | Remaining original budget; references |
|---|---|---|---|
| cSC4SubBudgetWindow;73,268/48 | `budget_window/dialog.rs` | Ordinance/neighbour/individual population and refresh; restore-record acquisition; UpdateValues; child/property lookup; setup/popup helpers remain same-class callbacks. Publish BW17 raw-input contracts, then implement these algorithms. | UNKNOWN. [Checklist](core-budgetwindow-ownership4.md#published-owner-inventory-audit), [spec](../specs/budget-window.md) §§4-8. |
| cHTMLDocument;38,218/162 | `html_document/{mod,parse,entities}.rs` | No missing published operation identified: report70 named+21 unnamed. Review HD5/HD6 choices and full checklist, then separately propose this map row if accepted. No glue prerequisite. | UNKNOWN; existing owner. [Report](core-htmldocument4.md), [spec](../specs/html-document.md) §10. |
| cSC4Watercraft;38,073/75 | `watercraft/` | Full animation/motion/position, collision/crash, route decoding/picking, far docking, player commands/following remain incomplete. Publish WC3-WC7 equations/ownership before completion. | UNKNOWN; parked, existing budget retained. [Checklist](core-watercraft4.md), [spec](../specs/watercraft.md) §§5-6. Report79 differs from current75. |
| cSC4Helicopter;35,775/72 | No dedicated core found | First candidate: parameters/PhysicsInit/InitInstance, NextFrame/rotor movement, AI controls, spotlight/states/player commands. Publish exact rotor/spotlight equations and inventory first; summaries cannot close owner. | UNKNOWN. [Spec](../specs/aircraft.md) §4. |
| cSC4Airplane;34,889/66 | No dedicated core found | First candidate: parameter sets/Init probes/effects, takeoff/land/taxi/goals, CalculateForces/UpdatePhysics/collision/state sounds/player translation. Publish flight-model and player parameters first. | UNKNOWN. [Spec](../specs/aircraft.md) §§3.1-3.4. |
| cSC4WinMapView;31,396/87 | No dedicated core found | First candidate: SetDataView/Reset/messages, grid-to-palette Update/interpolation/overlay, highlights/legend/scheduling. Publish legend and QV1-QV4 details first; exclude region-screen. | UNKNOWN. [Spec](../specs/map-view.md) §§3-9, [report](re-mapview.md). |
| cSC4ConstructionOccupant;26,539/79 | `lot_records/construction.rs` | Crane fit beyond boom pose, DivideLot, ground/roof props/crew, scaffold/completion interpolation and visual handlers remain delegated. Publish OP17 world-box writer and residual geometry contracts. | UNKNOWN; parent says round3 but later correction exists. [Correction](core-construction-contractfix4.md), [spec](../specs/lot-records.md) §9 and lot-developer §6.7.10. |
| cSC4WinSnapshotAlbum;23,300/47 | `snapshots/{album,info}.rs` | Exact captions, control/message/lifecycle, naming/collision suffix, ordering/selection and import/delete failure behavior. Publish SN1-SN9 before owner completion. | UNKNOWN. [Report](core-snapshots4.md), [inventory](../../engine/core/src/snapshots/OWNER_INVENTORY.md), [spec](../specs/snapshots.md). |
| cSC4ViewInputControlPlayerDrive;12,088/35 | `player_drive/mod.rs` | EnterDrivingMode dashboard/audio/settings, HandleKeyEffects, Init/ShowUI/Update and full entry/exit/selection remain incomplete. Publish DR1 and remaining drive contracts; retain bounded code parked. | **0**, original3/3. [Method table](core-player-drive4.md), [spec](../specs/drive-tool.md). |
| cSC4AudioAmbienceManager;11,298/57 | `audio/ambience.rs` | Report has all manager bodies; no missing published operation identified. Reconcile AA11 independent move-post vectors with existing source; AA10 absent-clock policy remains unknown. AA4/AA11 report labels lag answers. Separate layers do not block manager by themselves. | UNKNOWN; qualified mapping candidate. [Core report](core-audio-inputfix4.md), [answers](re-audio-final-gaps4.md), [spec](../specs/audio-ambience.md) §§3/9. |
| cLUAExtensionsCollector;9,045/30 | Advisor hooks/ScriptServer are different owners | First candidate: ordered38-entry simulation collector construction. Publish complete collector-owned callback inventory; cSCLua register_api/free callbacks cannot be borrowed for30-function closure. | UNKNOWN; ScriptServer cap does not transfer. [Spec](../specs/lua-host.md) §8.1. |
| cSC4AudioEventHandler;8,877/32 | `audio/event_handler.rs` | Full four-service Init/Shutdown undo, main/modal DoMessage/DoEvent and position extraction. Current helpers route types/post zero positions. Implement full lifecycle/dispatch over raw service/message/timer/position inputs. | UNKNOWN; existing partial. [Spec](../specs/audio.md) §7, [source](../../engine/core/src/audio/event_handler.rs). |
| cGZLua5Thread;7,303/87 | ScriptServer `foreign.rs` is only another-owner boundary | First candidate:97-slot stack/type/table/conversion surface, Ref/GetRef/Unref/GetUpValueIndex, protected-call/error/pop and precision guard. Publish lifetime/alias inventory, then independent VM wrapper. | UNKNOWN. [Spec](../specs/lua-host.md) §§4-6.2. |
| cSC4AudioSoundManager;6,909/49 | `audio/sound_manager.rs` | Nine-event dispatch: set-position/update/reset/enumerate/focus/stop-all remain absent/incomplete; UpdateAllSounds only counts; exemplar/focus and lifecycle need actual inputs/work. Publish unnamed-handler details then implement complete owner. | UNKNOWN; existing partial. [Spec](../specs/audio.md) §8.3, [source](../../engine/core/src/audio/sound_manager.rs). |
| cGZPersistDBSerialRecord;6,120/47 | `persistence_runtime/{serial,variant}.rs` | Ordinary typed/string/key/variant algorithms present. Exact allocation-threshold/malformed/partial memory effects and shared adapter/error state unrepresented/unknown. Resolve PR9/PR10 and assess explicit portable safe domains. | **0**, runtime3/3. [Report](core-persist-runtime4.md), [spec](../specs/persistence.md) §3. Report49/6172 differs from current47/6120. |
| cGZMessageServer;5,796/82 | `gz_messages/{server1,registry}.rs` | GZ8 cleanup visitation, GZ9 base-service lifecycle, GZ10 remaining-body inventory prevent blanket credit despite modeled send/post. Publish answers then use only inherited final completion round. | **1**, public logical2/3. [Budget/checklist](core-gz-messages-complete4.md), [spec](../specs/framework-gz-services.md) §4A. Report81 differs from current82. |
| cGZBufferResource;5,030/32 | `persistence_runtime/{resource,image}.rs` | Ordinary load/save/BMP/QFS present; malformed/negative size, allocation, derived-interface/failure cleanup remain qualified. Publish PR11/PR12; preserve padded-v1 PR3 choice. | **0**, same runtime family. [Report](core-persist-runtime4.md), [spec](../specs/persistence.md) §6. |
| cRZDirectory;4,385/20 | `rz_files/directory.rs` | No missing published operation identified: constructor/setters/enumeration/list recursion/search/create/remove/split/sort/rollback. Review both specs and RZ9-RZ11 choices then separately propose this row, excluding Path/DirectoryEntry. | UNKNOWN; no new engine round needed for mapping review. [Checklist](core-rzdirectory4.md), [framework spec](../specs/framework-rz-files.md) §6, [paths](../specs/paths.md) §3. |

## Recently landed portable owners: actual credit

Every unimplemented case below has `core=-` mechanically. Qualification determines whether that omission is supported or requires review; game glue alone is not an implemented obligation.

- **100:** [Directory](core-rzdirectory4.md) is a mapping-lag candidate. [Lighting](core-lighting-complete4.md)22,577 B retains LT12/LT13/numeric/lifecycle limits. [Model instances](core-model-instances-contractfix4.md) retain MI11 point/pick/precision limits; portable corrections are not all owner bodies.
- **101:** [Powerline](re-powerline-completion4.md), [draw context](re-draw-context-contracts4.md), [tools director](re-tools-director-questions4.md) add contracts to existing spec paths. Existence is binary, so extra sections yield no specified-byte gain. Tooling/docs have no engine owner credit.
- **102:** [View utilities/camera](core-view-utilities-complete4.md) explicitly partial and logical3 exhausted;50 named entries are not50 exact bodies. [Network model](core-networkmodel-contractfix4.md)18,805 B remains WIP/qualified effects. [Key accelerator](core-key-accelerator4.md)830 B has ordinary map operations but WT7/WT8 lifetime/QI/full-width ambiguities; separate resource owner3,105 B remains unspecified. [Dispatch](re-dispatch-completion4.md)/[renderer](re-small-renderer-contracts4.md) spec expansions do not add rows.
- **103:** [Input controls](core-view-inputcontrols4.md) lack full published inventories and exclude capped terrain/foreign owners. [Resource editor](core-resource-editor4.md) separates value models from native Init/render/catalog/control obligations. [Snapshots](core-snapshots4.md) retains SN gaps above. [Model/effects spec](re-model-effects-contracts4.md) expands already existing spec paths.
- **104/105:** [Application](core-application4.md) portable spans are not full cSC4App service ownership (AP3-AP9; independently assigned/region exclusions). [Paths](core-paths-contractfix4.md) PA4-PA8 correction explicitly bounded/logical3 exhausted, not full cRZPath886 B. [Powerline](core-powerline-contractfix4.md) remains parked/incomplete with withheld wire splits/relocation. Native command-line recovery does not add a second implementation.
- **106:** [Texture binding](core-textures-contractfix4.md) factory4,267 B retains TB1/TB2/TB8/normalization/ownership limits. Already credited FSH formats do not implement that factory.
- **107:** [WWW/FTP](core-ftp4.md)34,049 B remains unspecified/unimplemented; owned helpers return Unsupported and FINAL3/3 is exhausted/parked. [Lot editor](core-lotconfig-editor4.md)52,646 B and chooser3,852 B remain unspecified/unimplemented with LE3-LE6/native-owner limits; already credited LotConfigurationManager9,879 B and LotConfiguration9,580 B are different owners.
- **108:** [Player drive](core-player-drive4.md) gaps/cap above. [Persistence runtime](core-persist-runtime4.md) explicitly six partial owners; no13,721-B exact completed subtotal. Manager8,437 B was credited in100; KeyList separate. [LegacyControl](core-legacy-control4.md) current104,879 B/208 functions remains unspecified/unimplemented: report45 named=42 safe-domain+3 partial, twelve unknown interpreters/nineteen writers; report205/104,642 is a different inventory. Original3/3 exhausted; no widget/region borrowing.

Verification of **already implemented** FontSys8,358 B, Variant10,680 B, persistence manager8,437 B, command/language/command-line/landfill records and DBPF formats adds no implemented bytes. Constructed probes are not automatically the mapped same-process shadow key. Preserve [commands](verify-command-system4.md)'171 historical differences and both full-save stalls; [FontSys](verify-fontsys4.md)'constructed/natural distinction; [persistence](verify-persist-manager4.md), [variants](verify-variants4.md), [landfill](verify-landfill-records4.md), [language](verify-language-native4.md), [command line](verify-cmdline-native4.md)'scoped outcomes; and [DBPF](verify-dbpf-native4.md)'EX55 three differences, historical current40 ref4/OFF-turbo exit1 unknowns, bounded record/payload transport and exhausted3/3. None was promoted by this audit.

## Selection decision and checks

1. Separately review Directory then HTML complete-published-owner evidence: combined upper bound42,603 B (~0.763 percentage points), **not credited or promised**. No new engine family is needed merely to review the rows.
2. Reconcile AmbienceManager's answered AA4/AA11 versus source and retain AA10 product policy unknown. Do not require layer ownership or a game shadow for portable implemented status.
3. Publish GZ8-GZ10 then use MessageServer's one documented remaining completion round. Other existing UNKNOWN budgets require original reservation/cap reconciliation first.
4. Prepare first-owner MapView/aircraft/collector/LuaThread contracts specifically listed above before implementation. Summary-only physics or constructor-only inventories cannot close their bytes.
5. Do not refill exhausted WWW/player-drive/LegacyControl/paths/view/persistence families by renaming/restarting. Preserve parked Watercraft, region exclusions and all genuine failed/unknown outcomes.

**Supported:** exact totals/train deltas, explicit map blindness, current gaps, two strong mapping-lag candidates, documented caps. **Incomplete:** partial owners/full-body inventories. **Unknown:** open policy/native domains, unexplained historical failures, unrecorded family capacity. No accounting fix/reproducer/regression case is supplied because no actual accounting bug was proved.

Executed checks (read printed results, not exit codes):

- `python tools/coverage.py --out build-runs/owner-audit-report.md --owners build-runs/owner-audit-functions.tsv` printed `game code 29536 functions, 5584842 bytes; specified 57.8%, implemented 36.1%, verified 19.7%, replaced 18.1%`.
- Pinned107 invocation with `--regress C:/Users/tlori/FreeCity4/build-runs/regress-train107.out --out build-runs/owner-audit-pinned107.md` printed identical totals; normalized full report equals published.
- `python build-runs/audit_accounting.py` printed `PINNED107 REPORT MATCH; default108 same totals, different run evidence`, exact sums, train100+8437 and101-108 zero.
- `python build-runs/finalize_audit_evidence.py` printed `OWNER TABLE PASS: 18 distinct SPECIFIED/unimplemented owners; four areas; 378309 whole-owner bytes; 89 public source hashes` and budget assertions. It checks unique owners/statuses/references/areas and exact source provenance.
- Repeated default coverage printed the same totals. Byte comparison printed `DETERMINISM PASS: default report byte-identical across two runs`. Base-relative tracked-file assertion printed `PROTECTED PATHS PASS: only own lane report differs from reserved base; engine/tests/coverage inputs/generated reports unchanged`.
- `git diff --check` printed no whitespace errors (only the checkout's LF-to-CRLF warning). Cargo/clippy/CTest/game **n/a** for this docs/evidence audit; cited gates are other lanes' evidence, not reruns here.

Retained audit failures: initial default full-report equality assertion failed because default advanced to108; pinned107 resolved it without modifying reports. Initial public-reader cp1252 output failed, then UTF-8 configuration/rerun succeeded. Initial PowerShell brace-list command rejected before execution, then corrected. An attempted final report patch with delete/add of the same path was rejected before mutation and replaced with an update. Detached commits emitted packed-refs.lock permission warnings while actually creating commits; no shared refs/config changed.

Own milestones:7bde56f5 initial handoff;196146d6 reproduced accounting;e822a6a7 corrected complete-train comparison/substantive findings. Questions filed:none; existing IDs cited, not answered. Game requests:none/outstanding none. Evidence/scripts remain ignored under this worktree's build-runs; only this lane report is tracked.


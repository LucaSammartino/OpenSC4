| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-game-data-registry4 (first portable registry core) | Codex / `core-game-data-registry4` (engine lane) | **READY**, original round2/3: exact final engine2226d3b6 qualified; sole all exit0, Rust5388/0, clippy0, native4/4, CTest68/68; GDR1–5 retained; ABI unchanged; no game/native-owner credit | - |

## Round2 final-source qualification

This is the original family's second round (round1 consumed, round2 of3), not a
reset or new algorithm owner. No engine source, tests, ABI, CAPI, plugin, defaults
or shared state changed. Original report4fe93763 and committed/dirty work were
preserved. Commits260d6f99 and60b55315 record process checking and the historical
coordinator-action correction.

Fresh inspection05:18:46 found no own checker or listener on4590. CIM was denied;
psutil command lines were inspected independently of cwd access. No relevant
command-line errors remained;23 relevant cwd reads were denied and retained in
the raw inspection proof. Absence was not inferred from the old05:10:52 proof.

ONE canonical `python MAIN/.claude/lane-bin/qbuild.py --wt WT all` began at
2026-10-23T05:18:46.891365+01:00 on committed HEAD260d6f99, engine tree
2226d3b64adf716a335de644960e15ce494e7d58. Before any log overwrite,21 prior
R1 final/baseline/focused/failure evidence files were copied under
`build-runs/registry-r2-final/prior-evidence/` with `archive-sha256.json`.
Unmodified original raw journal and other failures remain available.

The sole pipeline was collector5928 → qbuild7056 → fc4verify22544 → optional
read-only stats27548. Exact creation/cwd/argv/parent and log-source evidence is in
`build-runs/coordinator-request-core-game-data-registry4-r2.md` and timestamped
`registry-r2-final/fresh-process-proof-*.json`. At05:24:05 stats27548 was still
alive, created1792729312.4687445, running `sccache.EXE --show-stats --stats-format
json` with parent22544 and this worktree cwd. Listener4590 belongs to daemon8408
(created1792729246.0032012, parent26616, cargo/bin cwd); its only observed child
was conhost23824. No daemon/compiler/validator kill, cache RPC, restart or
duplicate validator was performed by this lane.

ROOT then terminated ONLY optional read-only stats27548, created1792729312.4687445,
at05:24:16.413447–05:24:18.018816, exit15. The SAME fc4verify22544 and daemon8408
remained alive; qbuild7056 continued its original run. No daemon/compiler/validator
kill or restart occurred. Cache statistics unavailable; cause UNKNOWN. Copied
action proof `build-runs/coordinator-optional-stats-20261023-052416.json` retains
the exact process identity, parent/service continuity, no open outputs/writers
and unchanged native-log hash. This is separate from the round1 action on35076
and the earlier absent30532. No outcome was upgraded by optional-client termination.

The SAME original validator subsequently completed real native4/4 (64.8s),
CTest68/68 (135.0s), fc4verify PASS (349.5s); qbuild reports Rust exit0 with5388
passed/0 failed over23 result lines, clippy exit0/0 diagnostics, and overall PASS.
Collector terminal exit0 was retained at2026-10-23T05:26:33.200024+01:00.
This is the current source's fresh CTest result, not the prior04:54:55 log.
`pending-summary.json` preserves the earlier unqualified observation separately;
`summary.json` contains the actual terminal result. No duplicate gate/test was run.

`registry-r2-final/qualification.json` retains exact command/start/end/exit,
before HEAD260d6f992265b481db5d241b82700c8793bf4d4e and after
HEAD3fc08ce14f1b60f23faac83df303675a72c36c9b, engine/file SHA256 before/after,
and log hashes. Both engine trees are2226d3b64adf716a335de644960e15ce494e7d58;
all three file hashes match the final-formatted column below; both engine diffs
are empty and both source snapshots are clean. The collector confirms
`engine_equal=true`; the summary rechecks all log hashes unchanged.
The fc4verify tree-change annotation is explained by report-only commits, not
engine changes. Subsequent final-report edits also leave engine untouched.
`terminal.log` retains session65673 through terminal;05:26:38 process proof
shows no remaining own checker. The daemon listener remains; no shutdown was
requested. Exact final source is qualified; no second all for documentation.

Final SHA256 evidence (full configure/list hashes also in qualification.json):

- Rust log:0f8a4c120c486a9f48793a1bf0804f03f7f28574ef1b368c248a09ea6e95b1fa.
- Clippy log:d0c8d121b2745753559474b9b71f3073e3f3cb53bf6119d4aa711f00ef2ff81b.
- Native log:3d63fe3a18e0c541bb6b2e0e3c12287ddfacbf51cde8554d3f363609c3802219.
- CTest log:e96218a9d39785be01ab026a5a8d38ea3b858171eb57d09a9e6e082deb19a0e3.
- Terminal log:6ecf1a641cb027aa4be5f0f3126d09cbc0c0dca714d11962701de1cd0d82d3ad.

## Owned core

`engine/core/src/game_data_registry/{mod.rs,tests.rs}` and one additive lib export.
The core owns copied names, stable opaque 32-bit record tokens, separate exact
object/raw indexes, context/flags, parent/append-order children, validity value
scans and borrowed logical views. Supplied successful allocation tokens simulate
allocation identity; they are not native pointers, sequence IDs, factories or ABI.
Token reuse, foreign/stale records, unsafe callback/node inputs, null names/keys,
failed allocation, destructive reentry and unsupported lifecycle operations have
tagged domain errors, distinct from defined null/miss/false results.

The core resolves exact object pointers before requesting primitive IID1 QI,
searches the canonical output itself and releases every non-null QI output before
return. Node creation retains one key; duplicates retain nothing new. Leaves
retain neither key, callback nor context. Immediate, FIFO BFS and LIFO replay are
different algorithms. Removal collects first-to-last postorder; each removal
notification observes its own entry still indexed and earlier descendants absent;
only after that whole pass are node keys destroyed parent-before-child.

Typed/message lists are independent exact-pointer lists. New subscriptions have a
temporary search hold and net one list reference, replay only to the new target,
ignore callback false, and unsubscribe the first equal pointer. Ordinary events
go to typed targets only, including when the typed list is empty; message replay
uses raw standard-message allocation/setter/delivery/release primitives. Publisher
dispatch forwards raw keys rather than contexts, honors exchange read-only bit1,
does not infer stored variants or conversions, and preserves publisher Booleans.

Shutdown unregisters the framework before typed(1,null), clears indexes before
child destruction, releases subscriptions, preserves allocated root/name/object
but invalidates root membership. Destruction is separate and sends no synthetic
shutdown event. Logical registry/system/unknown views share a zero-start count;
ordinary Release0/1/>1 and deleting-wrapper low bit effects are modeled. Callbacks
receive only an immutable registry view: no claimed destructive-reentry fallback.

No App/Lua/GZ/Flora/CAPI/plugin/default/maps changes, game request, ABI export,
native replacement proof, whole-owner completion or coverage/default promotion.

## Measurement

- Baseline qbuild all exit0: Rust5372/0, clippy0 diagnostics, native build217/217,
  CTest68/68. The stats-only delay completed without lane intervention. Baseline
  logs copied to build-runs/baseline/. The tree-change flag was docs-only:
  engine tree c30f4d9ac1d78880e81cae198b7a461307350c58 stayed fixed.
- Independent literal ledger was fixed in build-runs/registry-literal-ledgers.md
  before production, SHA256 a3608e0632715d5cd435bf8e41c30060e50e046ccc2d926d33135006e42edc26.
- Single serial focused run:16/0,3328 filtered, exit0; registry-focused-cargo.log.
  The q.py Bash launch failed before cargo (Access denied); that failed transport
  log remains registry-focused.log. A local quiet Python adapter ran cargo once.
- Canonical qbuild all at7babaf6791b7e95307420d36c403a1d295d927c7, engine tree
  4f8e2f98cbca6a1222d565f43cb5197fd4acb304: qbuild exit0; Rust5388/0 over23
  result lines, clippy exit0/0 diagnostics, native build PASS4/4 edges (62.3s),
  CTest PASS68/68 (159.4s), fc4verify PASS (417.7s). No tree-change annotation.
  This is the original validator's completed result; the fresh final Ct log was
  checked, not the stale baseline file observed during the stats delay.
- Formatting: the check failed after moving files under engine/max_width120, but
  the shell command sequence continued into the canonical gate. Source stayed
  fixed until that validator exited. Only then commit9f4edebc applied the held
  rustfmt-generated patch to the two NEW files. Both new files now pass qbuild fmt.
  The required combined lib.rs/new-files check still reports190 diff lines in
  lib.rs module ordering. Read-only stdin checks with children skipped show the
  same preexisting first ordering difference in baseline e7b2ad83 (121 diff lines)
  and current lib (123); the only lib edit remains the required appended export.
  No mass sorting/reformatting of existing modules was performed.
- **Round1 ended WIP:** its one permitted canonical gate measured the pre-format
  source. Final formatted engine tree2226d3b64adf716a335de644960e15ce494e7d58
  differs. No duplicate focused/quick/all/cargo/CTest run was launched in round1.
  Round2's sole final-source qualification is recorded above. The round1 report commit
  changes documentation only relative to9f4edebc, not relative to the gate source.
- Historical stats provenance correction: ROOT terminated ONLY the optional
  read-only `--show-stats` client35076 (created1792727342.7140107), at
  04:52:15.453930–04:52:16.465343, exit15. The SAME fc4verify33964,
  qbuild26456 and daemon35116 remained alive. No daemon/compiler/validator kill
  or restart occurred; cache statistics were unavailable, cause UNKNOWN.
  That same original7bab validator subsequently completed real native4/4,
  CTest68/68, Rust5388/0 and clippy0. No outcome is upgraded by interruption.
  Earlier baseline stats30532 was already absent and ROOT took no action on it;
  that separate history must not be conflated with client35076. The original
  journal/raw outputs remain unchanged in build-runs/, with a prior-evidence
  archive under build-runs/registry-r2-final/. No lane process intervention.
  Historical PID/creation/argv/cwd/log metadata is preserved in
  build-runs/coordinator-request-core-game-data-registry4-{baseline,final}.md
  and coordinator-optional-stats-20261023-045212.json.
- Two initial apply_patch verification failures remain retained, alongside the
  pre-cargo q.py AccessDenied, initial formatting failure and continued pre-format
  gate, and preexisting lib.rs ordering drift. No failed result was erased or
  recast as a pass. Round2's CIM denial/cwd-access errors are retained separately.

Source SHA256 (all gate-time files remained unchanged until gate exit):

| File under engine/core/src/ | Canonical7babaf67 | Final formatted source |
|---|---|---|
|lib.rs|5fa6a8e3a0aa8031685d9e7789ba3365371cb6b68aa91f86fd18aff86a9d53c5|same|
|game_data_registry/mod.rs|6826c7c628047c7026d2fc524c315947eea2a5c518a2d671dad43acbe2b4c83d|f60200160d8c71741adda5f065f7d6bc30fea60b1a39300f803b95689af80ffd|
|game_data_registry/tests.rs|0d4c77cb1a39d3d89fbb7cedce47c343f7f4fa63edd3168223edcd4e3844fa60|c8584cea5020ab7df0d08b60e1db174ee6dee74538ce317d3751dcea203cea4e|

Milestones: e7b2ad83 initial handoff; c4c07520 contract/baseline delay; c8b461a4
owned core/tests/GDR5;7babaf67 focused results/canonical source; e0fb9a69 canonical
gate result;9f4edebc formatting cleanup. All commits carry the requested attribution.

Sixteen literal tests cover G1–G14, callback-false replay/broadcast versus visit
false, raw-key versus context, copied/same names with separate indexes, full-width
masks, QI success/failure output-release cases, 4→5→5→4 node refs, changing removal
membership, first-equal pointer removal, borrowed outputs with no AddRef, message
replay plus ordinary silence, shutdown root lifetime and destructor separation.
Expected ledgers are literal public-rule outcomes, not production-generated or
generic roundtrips. The first-equal duplicate-cell fixture is structural only;
public duplicate registration cannot construct it in the nonmutating domain.

## Published body/API inventory

This retains the published37 supported bounded /5 partial /6 foreign classification
of48 bodies/5807 bytes. It grants no blanket native-body or byte-coverage credit.
“Tested” below means finite portable effect/return cases, not native execution.

| Body | Published | Portable implementation / finite measurement |
|---|---|---|
|00473A40|S|Logical QI views/count; G14 tested; raw addresses unsupported|
|00473A80|S|Framework register primitive order; destructor test|
|00473AA0|S|Borrowed node; G8/G13 tested|
|00473AC0|S|Borrowed raw value; G3/G8 tested|
|00473AE0|S|Node predicate; G8/negative calls tested|
|00473B00|S|Borrowed copied name; G1/G8/G9 tested|
|00473B20|S|Primitive call forwarding; G7/G8 tested|
|00473B50|S|Full-width masks; G2/G6/G8 tested|
|00473B70|S|Context write incl0; G7/G8 tested|
|00473B90|S|Context read; G2/G7/G8 tested|
|00473BB0|S|Parent reads; G2/G8/G9 tested|
|00473BD0|S|Typed replay ignores false; G4/new-target tested|
|00473BF0|P|Raw standard-message primitive replay/release tested; allocation/accessor internals unsupported|
|00473C60|F|Generic node-index cell free/RB layout unsupported; safe collection primitive only|
|00473CA0|F|Generic leaf-index cell free/RB layout unsupported; safe collection primitive only|
|00473CE0|S|Primitive exchange/read-only forwarding; G6/G8 tested|
|00473D40|S|Typed append-order broadcast/message skip; G9/G11/G12 tested|
|00473DF0|S|Postorder collector owned; G9 tested|
|00473E60|S|Immediate creation-order visit/stop; G4/G5/G8 tested|
|00473F00|S|Index value-membership scan; G8/G9/G12/G13 tested|
|00473FB0|S|First exact typed unsubscribe; G10 tested|
|00474020|S|Exact message unsubscribe; new-target test|
|00474090|F|Generic node RB erasure/rebalance unsupported; safe map primitive only|
|00474150|F|Generic unsigned key bounds unsupported as native helper; exact safe map keys only|
|004741B0|F|Generic leaf RB erasure/rebalance unsupported; safe map primitive only|
|00474270|S|Core exact/QI fallback search and output release; dedicated ledger|
|00474310|F|Generic raw-pointer vector adapter unsupported; safe Vec primitive only|
|00474350|S|Parent-first object-key release, raw leaves unretained; G9/destructor tested|
|004743E0|S|Exchange flags/copy/key initialization; G3/G6 tested|
|00474460|S|Call flags/copy/key initialization; G1/G6 tested|
|00474580|P|Owned destructor effects tested; native/base/allocation teardown unsupported|
|00474670|S|Logical shared QI view modeled; raw+4 adjusting thunk unsupported|
|00474680|S|Logical shared QI view modeled; raw+8 adjusting thunk unsupported|
|00474690|S|Logical deletion effect modeled; raw adjusting thunk unsupported|
|004746A0|S|Framework-unregister/event/clear/preserved-root order; G12 tested|
|00474780|S|Collect/notify/unindex/destroy/unlink algorithm; G9 tested|
|004748F0|P|Logical assembly from supplied successful root tokens tested; raw allocation/partial failure unsupported|
|00474A30|S|Logical low-bit free-allocation effect; dedicated leaf/deleting test|
|00474A50|S|Owned node creation/duplicate unchanged; G1/G2/G13 tested|
|00474B30|S|Owned exchange creation/duplicate null; G2/G3/G6 tested|
|00474C50|S|Owned call creation/duplicate null; G1/G6 tested|
|00474D70|S|Object-key lookup/removal wrapper; G9/G13 tested|
|00474DA0|S|Raw-key lookup/removal wrapper; G11/raw-leaf tested|
|00474DE0|S|Reverse-sibling DFS replay owned; G4/new-target tested|
|00474F70|S|Typed hold/search/net1/replay registration; G10/new-target tested|
|00475030|P|Message list ownership/replay primitives tested; native message accessor/allocation internals unsupported|
|004750F0|S|FIFO including start, false before child enqueue; G4/G5/G8 tested|
|00475630|P|Null dump no-op tested; non-null diagnostic formatting tagged unsupported GDR4|

The additional node constructor004744E0 and subscription-list clear00473EC0 have
logical owned effects covered by creation/shutdown/destructor tests; no additional
48-body credit. Dump visitor00475330 stays unsupported. Shared AddRef/Release,
raw lookup and root access are logical portable operations, not newly attributed
native bodies. No private slots/layout, path lookup, name uniqueness/casefolding,
numeric conversion, factory/native construction or Lua mirror is inferred.

## Questions and next steps

Filed GDR5 in docs/specs/questions/game-data-registry.md: anonymous replacement key
collision with an existing raw leaf key. The portable model tags this unsupported
rather than inventing overwrite/native-false semantics. GDR1–4 remain unsupported.
The sole round2 validator's actual terminal result qualifies the final formatted
source. Own normal merge queue is ready; no outstanding checker/game request.
No automatic third/fourth round, new tests, native/game probes or owner expansion.
A later authorized lane owns interoperability/wiring or further native evidence.

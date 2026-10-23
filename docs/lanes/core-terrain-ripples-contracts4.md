| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-terrain-ripples-contracts4: bounded WR8 contracts | Codex / core-terrain-ripples-contracts4 (engine) | ready: canonical cargo5513/0, clippy0, nativePASS2/2, Ct68/68, terminal0; WR11 open; FINAL child2/2 family3/3 exhausted | 1. Coordinator review and normal landing of bounded scope; no further lane round |

## Result and implementation

Bounded substantive WR8 source is committed at `d130ebad`; inherited portable
execution is recorded at `06bff0c7`. This lane is **ready for bounded source
review and landing**: the first canonical all reached actual terminal PASS on
unchanged committed engine content. Gate evidence was committed at `1e669e8f`.
No whole-owner, default, native/runtime equivalence, game or byte-coverage credit
is claimed. This final invocation changed only this report; no source fix or
algorithm expansion was needed.

`retained.rs` replaces the provider-decoded/all-properties/fixed-five exemplar
read. Core owns suffix 00..0D ordering, numeric source/target gates, shared
`variant::Elem` conversion, temporary destruction and output only on success,
unsigned minimum clamp, raw pointer/count copies and first-five transforms.
All six elements survive copying; absence retains and retransforms arrays.
Missing whole exemplar returns before clamps/transforms/releases. Invalid backing
and unchecked posttransform indices have typed Unsupported results, with earlier
mutations retained. Foreign sources can supply only an individual primitive
conversion result, never final tuning, arrays or process state.

`RippleProcess` is explicit shared storage held through Arc/Mutex by WaterRipple.
Init's initialized guard precedes the shared flag; flag/callback registration
precede reads. Reset only clears the flag. Shutdown/re-Init skips reads while the
flag is set, and preserves first-Update/zoom/rebuild state. Getter snapshots use
the shared mutex, so another instance's reload is visible. Logical reference
count/QI/Release decisions in `lifecycle.rs` use semantic interface tokens rather
than invented COM addresses. Rust Drop memory safety is separate from explicit
native event chronology; no destructor Shutdown is fabricated.

`storage.rs` owns 24-byte vertex, 16-byte rectangle and 8-byte point records,
raw copied tails, distinct nested buffers/nodes/sentinels, prefix reuse, exact
self-assignment, capacity retention and exact growth-before-free. Fixed-position
range insertion, forward erase including its empty-range output iterator,
nonempty pop, sentinel-retaining clear and advancing shore range assignment are
implemented with stable logical identities and allocation/free ledgers. Geometry
storage has no terrain/texture AddRef/Release operation. General overlap,
self-insertion, allocation failure and outer-vector insertion remain outside
the implemented contract.

The older `RippleTuning` remains explicit input to isolated geometry kernels,
not process tuning or a missing-property fallback. Full WR4/WR5 operations retain
their named ContractGap results; this round did not add partial successful walks.

## Measured verification

| Check | Actual result |
|---|---|
| Canonical all Rust | **5513 passed, 0 failed, 0 ignored**, 26 summaries; workspace tests |
| Terrain-ripples tests | **51 passed:** 37 retained tests and 14 new independent WR8 tests |
| Clippy | **exit 0, diagnostics 0**, workspace -D warnings; declared workspace MSRV 1.75 |
| Owned-file fmt wrapper | Inherited **exit 0**, eight .rs files using absolute paths; source unchanged |
| Git whitespace check | **exit 0** |
| Native CMake | fc4verify **PASS, 2/2 edges**, 1.6 seconds; successful native stage, not inferred from links |
| Full CTest | **68/68 passed**, zero failed; log 103.68 seconds, wrapper 104.1 seconds |
| Canonical wrapper terminal | **exit 0, qbuild PASS**, fc4verify PASS in 108.2 seconds |
| Historical R1 quick Rust | 3416 + 21 + 13 + 19 + 0 = **3469 passed, 0 failed** |
| Historical R1 quick terminal/native RC | **UNKNOWN**, old handle74315 unavailable in this invocation |
| Game/shadow | n/a: core-only, manifest game=false; no requests |

All 26 Rust summary pass counts, in log order:
`540, 3, 1, 31, 27, 40, 7, 2, 1232, 3416, 21, 13, 19, 147, 0, 2, 1, 3, 1, 1, 1, 5, 0, 0, 0, 0`.
Every summary has zero failures/ignored tests. All 51 named terrain-ripples
outcomes end in `ok`; exact summary tuples and names are in
`build-runs/r2-rust-results.json`.

The literal expectations were written before production comparison. New cases
cover .75-to-bool false, negative-float unsigned wrap, high-half Uint64 truth,
all numeric source tags, absent minimum clamp, sixth-element/source-bit retention,
repeated absent 02/0B transforms, whole-exemplar/property differences, temporary
destruction on primitive false, shared process chronology and nested deep-copy,
identity/capacity/free-order/range behavior. The module conformance ledger marks
only those executed bounded domains; the unsafe texture failure path remains
excluded. Historical 4314/67 counts are retained as history, not this gate's result.
The old atomic-missing-exemplar expectation is explicitly marked DIFFERS under
the newly published §2.4 normal Init contract.

First fmt wrapper attempt used relative paths and returned missing-file errors;
the corrected absolute-path wrapper check passed. The q.py formatter attempt
could not launch Bash (E_ACCESSDENIED), so the owned files were formatted in
place directly before the quick build. No baseline/redundant quick was run.

### Historical R1 evidence retained without a terminal upgrade

Before canonical paths were overwritten, 26 inherited files were archived in
`build-runs/inherited-r1-final-child2/`, including source, manifests, copied
host admission, logs and the stall proof. Its `sha256.json` records all content.
The historical observed chain in `quick-stats-block-proof.json` is:

- qbuild PID **30108**, parent **21984**, created **2026-10-23T05:20:34.317975Z**;
- fc4verify PID **8540**, parent **30108**, created **05:20:34.358725Z**;
- sccache client PID **38160**, parent **8540**, created **05:24:23.020547Z**;
  argv `C:/Users/tlori/.cargo/bin/sccache.EXE --show-stats --stats-format json`;
  cwd this worktree; TCP **127.0.0.1:55369 → 127.0.0.1:4760 ESTABLISHED**.

Archived output evidence: `fc4verify-quick-test.log` contains all five Rust
summaries; archived `fc4verify-build.log` ends with `[216/217] Linking CXX shared library
FreeCity4.dll` and `[217/217] Linking CXX executable fc4_plugin_load_test.exe`.
Those link counts are **not** treated as a successful native RC or terminal gate.
The copied host proof at 06:41:03 BST found the known old processes absent,
own port4760 services empty and six output write-sharing probes clear/absent.
This admits a new operation; it does not establish historical exit or cause.
Root did not interrupt old client38160. The old terminal remains UNKNOWN.
No client cancellation, duplicate build, cache RPC, kill/restart, environment
or configuration change was performed by this invocation.

Source engine tree for quick was `4f2cced6ac62bbae891400e45d1865e8ca7d8922`
at `d130ebad`; the later engine change is the conformance result ledger only.
The known archived quick log hash is
`8f544565d98cbcdccd5cdff8b1a00efe557818558f533f597a07d061856e6183`;
archived native progress log hash is
`69176633439f1ffaf01cf4fec7b5037b97b30f65d478ada78f1768f31a8ab949`.
Both were verified before the canonical run.

### Canonical operation and stable content proof

One canonical `qbuild all` ran on committed HEAD
`d21121524a58dbaf579eeb3dc3dcf54f2958d041` (the inherited engine plus report only).
Session **96723**, qbuild PID **3480**, parent **18676**, creation
**2026-10-23T06:00:56.999592Z**, argv `python qbuild.py --wt <this worktree> all`,
cwd this worktree, was collected through actual terminal exit0 with
PowerShell `exit $LASTEXITCODE`. No observer intervention was required.

`build-runs/r2-before-all.json` and `r2-after-all.json` contain all tracked
engine file hashes, manifest hash and log hashes. Engine tree before/after:
`938080c528fea6f31ce22b6a0e982045cc20c4b5`. Aggregate engine content SHA256:
`2420031778762d81d77754778de8504a4450719cf7e331f58a37d341e5346215`.
The complete file maps compare equal. Later commits change only this lane report;
the implementation is identical to the qualified source.

Canonical log SHA256 values:

- `qbuild-test.log`: `e0e747a3a500c8de474538f63a905ce4078f5f2625379ef21c1a021f19f55d7b`.
- `qbuild-clippy.log`: `0a18f3639b7256eb2158ef73080cc7c7717c535a33c64179da9437ec80578140`.
- `fc4verify-build.log`: `c8f2a21fdcaa188bbf69b288e0abbfd25372e51942095ad8a4d6edf51a9e905f`.
- `fc4verify-ctest.log`: `c4985a435b44acfdbeb1cdd023ed4415453cba175e04219c52bab47ea42dec7a`.

`build-runs/r2-terminal.json` preserves the terminal stage results and process
identity. An optional own-process diagnostic hit the installed psutil API's
missing `net_connections` method; it did not affect the gate. The helper was
corrected to `connections`, without rerunning it against the finished operation.
Git commits succeeded despite a read-only shared `packed-refs.lock` error;
no shared refs/config/maintenance action was attempted.

## Questions and remaining work

**WR11**, appended to `docs/specs/questions/water-ripples.md`, asks for the stored
binary32 degree-factor bits. The core takes a raw factor parameter; tests use
literal 0.5 and make no native-value claim. Period bounds use exact product/floor
for period <=2^24; larger values and unproved inherited numeric modes are explicit
Unsupported domains. WR1–3, full WR8 catalogue/Pick semantic types, foreign
conversion/error/allocator/reentry and PC/RC residuals remain open. The 69-vs-65
catalogue is not reconciled or credited.

Coordinator review and normal bounded-source landing remain. There is no
remaining lane gate failure. Native equivalence and game/shadow proof are n/a.
Parent stays frozen and family3/3 is exhausted: no extra round, restart, rename,
sibling or parent resumption is requested. Open semantic domains above remain
open after this bounded qualification.

## Admission and scope

Detached worktree reused, no branch/ref/config updates. Root's actual immutable
reservation is copied in `build-runs/evidence-run4/`; root DRAFT review_note records
fresh FULL canonical terrain-ripples engine BOTHdeps GO0, copied in
`core-host-admission.md`. The current own R2 manifest is round2, core/engine,
this exact path; copied final host proof includes full canonical scope and both
dependency admissions. Codexguard status remained within budget. Parent remains
frozen at used1/limit1; FINAL child2/2 is original logical3/3, family ceiling3. No
isolated global preflight rerun, parent/sibling resume, reset or guard change.
No CAPI/plugin/ABI/maps/default/game/native equivalence or foreign-owner edits.

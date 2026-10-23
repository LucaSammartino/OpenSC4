| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-terrain-ripples-contracts4: bounded WR8 contracts | Codex / core-terrain-ripples-contracts4 | in progress: FINAL child2/2, original family3/3; inherited evidence archived | Run one canonical all on frozen source |

## Handoff

- Goal: qualify existing bounded WR8 implementation; FINAL child2/2, family3/3, no further round.
- Source: clean inherited 9731b69b; engine tree 938080c528fea6f31ce22b6a0e982045cc20c4b5.
- R2 authority: own round2 manifest, core/engine path; codexguard within budget; copied host proof admits drained operation.
- Preserved 26 inherited files in build-runs/inherited-r1-final-child2; sha256.json records exact content.
- Both specified quick/native log hashes matched; old quick handle unavailable in current invocation, terminal/native RC UNKNOWN.
- Engine content SHA256 before all: 2420031778762d81d77754778de8504a4450719cf7e331f58a37d341e5346215.
- Next: canonical qbuild all, retain same operation to terminal, record before/after content and log hashes.
- No source changes, no quick rerun, no game requests; WR11 and other inherited limits stay open.
- Below is the inherited R1 report; it is historical until replaced by the final R2 report.

## Result and implementation

Bounded substantive WR8 source is committed at `d130ebad`; measured portable
execution is recorded at `06bff0c7`. This lane is **not ready to merge** because
the required final gate has no terminal evidence. No whole-owner, default,
native/runtime, game or byte-coverage credit is claimed.

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

## Measured verification and gate limitation

| Check | Actual result |
|---|---|
| Quick Rust summaries | 3416 + 21 + 13 + 19 + 0 = **3469 passed, 0 failed** |
| Terrain-ripples tests | **51 passed:** 37 retained tests and 14 new independent WR8 tests |
| Owned-file fmt wrapper | **exit 0**, eight .rs files using absolute paths |
| Git whitespace check | **exit 0** |
| Native CMake | Log reaches final DLL/executable link commands; terminal native RC unavailable |
| Quick terminal result | **unavailable**, active handle 74315 blocked in post-build stats |
| Canonical all / clippy / full CTest | **not run** while the existing quick command remains active |
| Game/shadow | n/a: core-only, manifest game=false; no requests |

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

### Exact active-client evidence for root

Full machine-readable proof is preserved in this worktree's
`build-runs/quick-stats-block-proof.json`; capture script is
`build-runs/capture-build-proof.py`. The observed chain is:

- qbuild PID **30108**, parent **21984**, created **2026-10-23T05:20:34.317975Z**;
- fc4verify PID **8540**, parent **30108**, created **05:20:34.358725Z**;
- sccache client PID **38160**, parent **8540**, created **05:24:23.020547Z**;
  argv `C:/Users/tlori/.cargo/bin/sccache.EXE --show-stats --stats-format json`;
  cwd this worktree; TCP **127.0.0.1:55369 → 127.0.0.1:4760 ESTABLISHED**.

Output evidence: `build-runs/fc4verify-quick-test.log` contains all five Rust
summaries; `fc4verify-build.log` ends with `[216/217] Linking CXX shared library
FreeCity4.dll` and `[217/217] Linking CXX executable fc4_plugin_load_test.exe`.
Those link counts are **not** treated as a successful native RC or terminal gate.
The same quick session handle **74315** was collected in <=60-second waits;
diagnostic handle 55269 completed. No client cancellation, duplicate build,
cache RPC, kill/restart, environment or configuration change was performed.

Source engine tree for quick was `4f2cced6ac62bbae891400e45d1865e8ca7d8922`
at `d130ebad`; the later engine change is the conformance result ledger only.
There is no claimed canonical-all before/after fingerprint because all has not run.

## Questions and remaining work

**WR11**, appended to `docs/specs/questions/water-ripples.md`, asks for the stored
binary32 degree-factor bits. The core takes a raw factor parameter; tests use
literal 0.5 and make no native-value claim. Period bounds use exact product/floor
for period <=2^24; larger values and unproved inherited numeric modes are explicit
Unsupported domains. WR1–3, full WR8 catalogue/Pick semantic types, foreign
conversion/error/allocator/reentry and PC/RC residuals remain open. The 69-vs-65
catalogue is not reconciled or credited.

Root must adjudicate the stuck stats client, collect existing quick handle 74315
to terminal, then run **one canonical qbuild all on stable committed source**,
record every Rust summary/clippy/native RC/full CTest/terminal and engine/content
before-after proof. No lane-side restart or extra family round is requested.

## Admission and scope

Detached worktree reused, no branch/ref/config updates. Root's actual immutable
reservation is copied in `build-runs/evidence-run4/`; root DRAFT review_note records
fresh FULL canonical terrain-ripples engine BOTHdeps GO0, copied in
`core-host-admission.md`. Own manifest is round 1, core/engine, this exact path;
parent remains frozen at used1/limit1, child logical2of3, family ceiling3. No
isolated global preflight rerun, parent/sibling resume, reset or guard change.
No CAPI/plugin/ABI/maps/default/game/native verification or foreign-owner edits.

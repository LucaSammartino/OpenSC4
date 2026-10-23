| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-rz-sparsematrix4: first portable sparse matrix | Codex; core-rz-sparsematrix4 | in progress: literal review complete; 21/21 new tests passed, quick native still active | Collect same quick handle; then one canonical all on stable committed engine |

## Handoff

- Goal: qualify existing bounded kernel/tests, then one stable committed-source all gate.
- Start: clean detached b94fe174; engine f4bb6e0daecb9d7a0b248137aef2736c590ec0b6; manifest confirms engine lane, own worktree, round 2.
- Original evidence: build-runs/r2-original-evidence/manifest.json, seven verified files, 357186 bytes; originals copied before any fixed-log overwrite.
- Old gate predates implementation: Rust 5372/0, clippy clean, native terminal UNKNOWN, CTest unrun/unknown; no new-source pass.
- Copied coordinator 04:37:56 proof: old PIDs absent, own 4713 listener/reference hits absent; system AccessDenied entries remain unknown. Cause unknown; root did not terminate old clients. Do not recollect absent session 83439 or rerun baseline.
- R2 q.py read-only diagnostic failed Bash E_ACCESSDENIED; use PowerShell/Python, no repeat Bash diagnosis.
- Literal qualification completed before execution: build-runs/r2-test-qualification.md maps SM-V1–13 to independent tables/traces across all 21 tests. No expectation or production changes needed.
- First actual new-source result: fc4verify-quick-test.log 3349 passed/0 failed/0 ignored, 21 sparse tests explicitly ok; dependencies compile complete. qbuild quick handle 49758 still running native build; collect same handle, no duplicate quick/all.
- Measurement at 03:46:32 UTC: build-runs/r2-quick-progress-measurement.json; own listener 4713 PID 28648; own qbuild/fc4verify/native chain retained there. Not a terminal native result. Old qbuild-test/clippy logs still baseline, not new-source evidence.
- Next exact step: collect same quick handle and measure terminal; stable engine remains f4bb6e0d. Then one canonical all on committed source, preserving before/after hashes and actual terminal evidence.
- No game request; standalone core, ABI unchanged. SM1–5 remain unresolved and excluded.

## Report

First distinct standalone sparse matrix core, implemented from landed
`rz-sparse-matrix.md` §§1–9 and open SM1–SM5. Implementation commit
`17001ed9b898239eb61933ab1de7187481e47da6`; initial handoff `22b7bff7`.
Only engine changes are the new module, its tests, and an additive lib export.
This is WIP because validation is blocked; authored tests are not measured passes.

### Implemented bounded behavior

- Explicit signed inclusive caller bounds and validated sparse fixture state.
  This is a portable API choice, not a native constructor/default/copy/resize.
  Nonempty rows and unique coordinates are maintained in the normal domain.
- Separate one-word PointerMatrix and ten-word ExtraMatrix types, with opaque
  pointer integers and borrowed logical payload views. Lookup miss preserves
  output; None tests existence. No native pointer lookup alias is claimed.
- Core-owned signed selection, row-major ordering, wrapping u32 counts,
  zero-valued occupancy, inclusive/inverted ranges and 12-byte logical records.
  Optional range output counts only; portable capacity checks are separate from
  native behavior. No capacity error is claimed for the original.
- Bounds rejection precedes all payload access. Existing replacement writes
  old output before reading input, including a genuinely shared independent
  word/buffer through safe primitive WordIo tokens. New-cell/new-row insertion
  leaves old output untouched and increments counts on normal storage return.
- RemoveCell hit copies output before erasure/count changes and erases an empty
  row; misses leave output untouched. Wrapped start+count removals select signed
  half-open keys without renumbering or count<=0 shortcuts. Column removal
  captures the next row before deleting an empty row.
- Extrema sentinel initialization, min-before-max alias stores, backward final
  row search, first normal maximum tie and last INT_MIN sentinel tie.
- Borrowed fixed-column iteration skips holes and visits zero payloads. Opaque
  stable identities model logical alias relationships, not native addresses.
  Identity equality ignores matrix; two null-cell ends compare equal regardless
  of row. End increment is explicitly unsupported; Rust borrowing prevents
  mutation during live iteration. No invalidation guarantee is inferred.

### Validation and blocker

Baseline qbuild all started on `22b7bff7`, before engine changes. Its Rust phase
has 23 result summaries: **5372 passed, 0 failed, 0 ignored**. Clippy finished
without diagnostics. Native build log ends at 217/217 linking. **There is no
native terminal RC, CTest result, or complete qbuild exit; this is not a gate pass.**

Read-only process inspection confirms qbuild PID 32304, creation epoch
1792725512.9155433, waiting through fc4verify PID 24192, creation
1792725660.813306, on sccache stats PID 21872, creation 1792725814.095833.
Stats argv is `C:/Users/tlori/.cargo/bin/sccache.EXE --show-stats --stats-format json`.
All three cwd values are this worktree. Baseline exec session is 83439; leave it
running and collect its terminal result after coordinator resolution. No second
quick/all/cargo/CTest/stats RPC was launched; no validator was cancelled.

Exact argv, log sizes/mtimes/hashes and source identities are recorded in
`build-runs/coordinator-request-core-rz-sparsematrix4.md`. Baseline engine tree
is `c30f4d9ac1d78880e81cae198b7a461307350c58`; implementation engine tree is
`f4bb6e0daecb9d7a0b248137aef2736c590ec0b6`. Baseline evidence does not validate
the new source. Full original logs remain in build-runs, unoverwritten.

New-source qbuild fmt **exit 0**, both new Rust files, using absolute paths and
repository formatting. Earlier fmt failed first because relative filenames were
resolved outside the worktree, then because stdin rustfmt missed engine settings;
both corrected. A q.py diagnostic command failed with Bash access denied;
subsequent diagnostics used local PowerShell/Python. These are retained failures,
not successful checks. Windows CIM process access was denied; psutil supplied
actual PID/creation/cwd/argv evidence. Git commits succeed detached despite a
packed-refs.lock permission diagnostic; no shared refs/config were changed.

21 authored literal tests cover SM-V1–13 and discriminators for optional lookup
and range outputs, signed inverted intervals, negative-count endpoint wrap,
multirow successor order, nonalias replacement, new cells/rows, borrowed address
identity, independent shared 40-byte buffers, missing output preservation, and
portable validation errors. Expected arrays and word-access/event logs are
literal deductions written independently of production selection helpers.
**New focused tests and canonical final all remain unexecuted.**

### Per-entry ledger

Effect tests below are authored and pending execution; no native effect measurement
or owner-coverage credit is claimed. “Bounded implemented” is source status only.

- 0095a0d9: partial/unknown; lifetime wrapper omitted (SM3).
- 009a5bf2: foreign/misattributed record initializer; omitted.
- 009a5c36: foreign/misattributed record initializer; omitted.
- 009a7577: bounded implemented; equality effects SM-V11 pending; portable bool.
- 009a9122: bounded implemented; begin effects SM-V3 pending.
- 009a9225: bounded implemented; extrema effects SM-V9/V10 pending.
- 009a9280: bounded implemented; inclusive count effects SM-V1 pending.
- 009a92db: bounded implemented; ordered range effects SM-V2 pending.
- 009a9a89: bounded implemented; column extrema/alias effects SM-V9 pending.
- 009a9ada: bounded implemented; row extrema/alias effects SM-V9 pending.
- 009a9b7d: bounded implemented; row occupancy effects SM-V1 pending.
- 009a9b9e: bounded implemented; column occupancy effects SM-V1 pending.
- 009a9bd7: bounded implemented E logical address lookup; optional output and
  miss-preservation effects pending; pointer specialization native alias unknown SM5.
- 009ab6b4: partial; bounded output/erase/count effects SM-V12 pending;
  foreign destruction/exception behavior unknown SM2/SM3.
- 009ab716: partial; bounded wrapped row removal effects SM-V6/V7 pending;
  foreign eraser exceptions unknown SM2.
- 009ab75e: partial; bounded wrapped column/successor effects SM-V8 pending;
  foreign eraser exceptions unknown SM2.
- 009ac02f: partial; bounded ten-word owned copy effects SM-V13 pending;
  foreign construction/allocation/copy/error guarantees unknown SM2/SM5.
- 009ac157: partial; bounded one-word insertion effects SM-V4/V5 pending;
  foreign construction/allocation/copy/error guarantees unknown SM2.
- 009a8fec: bounded implemented; increment effects SM-V3 pending;
  invalidation/end behavior unsupported SM4.
- 0095a087: foreign AutoPtr release, omitted.
- 0095a0aa: foreign AutoPtr destructor, omitted.
- 0094c8f5, 009a947f, 009a94a6: foreign/shared tree and exact lookup helpers;
  portable maps store nodes only; core owns membership and traversal decisions.
- 0040d4c0, 0040d500: foreign/shared successor/predecessor; native layout omitted.
- 009abc11, 009599a9, 009a94ca, 009abb8d, 009ab2b6, 009a9c16, 009a9427:
  partial foreign dependencies; failure/lifetime semantics unknown and unsupported.
- 009a9c2f: foreign shared construction; no native defaults inferred.

Public 39/3358 attribution remains **UNRECONCILED** (SM1). The observed Windows
18 matrix entries/1784 bytes + iterator/82 + two foreign entries/82 = 21 named
entries/1948 bytes is not a whole-owner denominator. No factory, native-complete,
coverage, default or specialization promotion; SM1–SM5 remain open. No new
question was required: all excluded behavior is already covered by those IDs.

### Game and next steps

Game: n/a, standalone core only; no plugin, WinGrid, CAPI, ABI, config, maps,
factory or shadow wiring. No game request or exact-native comparison claimed.

Coordinator must resolve the owned stats stall and collect the original validator
terminal. Then run focused qbuild quick, fix any compiler/test/clippy failures,
commit stable source, and run ONE canonical qbuild all with true Rust/clippy/
native RC/CTest/log-hash/source proof. Recheck fmt if source changes. Update this
report and WIP queue only after that evidence exists. No landing while WIP.

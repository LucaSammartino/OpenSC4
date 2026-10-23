| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-rz-sparsematrix4: bounded sparse matrix, original R2 | Codex; core-rz-sparsematrix4 | WIP: quick PASS/0; corrected-source Rust 5393/0 incl. 21 sparse tests, clippy clean; canonical all live in stats | Root decision on exact stats PID 4940; preserve/collect same all session 14747; native terminal/CTest unknown |

## Report

Original round 2, not a new lane/family. Started clean at detached
`b94fe174f1c65a9a3b2a0dac5abfa74e92b69c6d`, engine tree
`f4bb6e0daecb9d7a0b248137aef2736c590ec0b6`. Own manifest confirms engine
lane/worktree/ROUND2. Read copied admission, own R1 report, landed public
dependency/spec and SM1–5. No branch/ref, coordinator, cache, daemon, tools,
shared guard, plugin, CAPI, ABI, or existing RZ implementation changes.

The bounded kernel and its 21 tests were implemented in `17001ed9`. R2 first
qualified and executed them, then fixed one actual warning in `4c22040f`.
The new committed engine tree is `57061af0d159dc2c7803383c212fb04e8ca0c300`.
The corrected source passed workspace Rust tests and clippy during the ONE
canonical all launched on clean `e7ed6166`; native release compilation and
linking completed, but the validator is still waiting in its read-only stats
child. No complete all/native terminal or CTest success is claimed: WIP.

### Qualification and measured results

Before execution, read all 21 tests against published §§2–9 and SM1–5.
`build-runs/r2-test-qualification.md` maps SM-V1–13 to independent literal
expected values/traces. Expectations were not changed or fitted to output.
WordIo supplies only primitive independent-buffer reads/writes, no selection.

First new-source `qbuild quick` on engine `f4bb6e0d`:

- Rust **3349 passed, 0 failed, 0 ignored**, including **21 sparse tests
  explicitly ok** in `fc4verify-quick-test.log`. Dependencies compiled.
- Signed negative keys, inclusive/inverted ranges, row/column order, 12-byte
  value records, zero-valued occupancy and count-only output are distinguished.
- Shared independent single-word and 40-byte buffer traces require all old
  stores before input reads, restoring old values to both buffer and cell.
- Missing lookup/removal and new-cell/new-row insertion preserve outputs;
  bounds rejection performs no payload access.
- Removal tests distinguish wrapped start+count from a count<=0 guard, retain
  unshifted keys, and capture successor before deleting an empty row.
- Extrema test min-before-max alias stores, absence sentinels, last-row INT_MIN
  ties and first-row ordinary maximum ties. Iteration skips holes, visits zero
  payloads; end equality ignores matrix/row, live equality requires row+cell.
- Same quick session 49758 eventually returned **PASS/exit 0**, native build
  4/4 edges (163.1 s), fc4verify 474.6 s, qbuild PASS. Its explicit tree-changed
  warning limits it to the pre-correction compilation; CTest not requested.
  Exact terminal: `build-runs/r2-quick-terminal.json`. Ending cause of the prior
  stats wait is unknown; no lane process action was taken.

Actual warning: AtomicU64::fetch_update is deprecated on installed Rust;
try_update is newer than workspace Rust 1.75. `4c22040f` uses an equivalent
relaxed compare_exchange_weak loop with checked increment, preserving opaque
identity allocation/exhaustion. No algorithm or test expectation changed.
Both own Rust files pass qbuild fmt exit 0 after correction. Once quick actually
returned, ONE canonical all was launched on committed corrected source. No
duplicate quick/all was started and no compiler/cache/client was acted on.

### Canonical all on corrected source: partial, not PASS

Started on clean `e7ed616663aed9ae3f07d79c2d9b2ccc3f4cb803`, engine
`57061af0d159dc2c7803383c212fb04e8ca0c300`; before measurement is
`build-runs/r2-before-all-measurement.json`. No tracked changes or commits were
made during its actual Rust/clippy/native compilation. Report-only commits
after the live wait do not change the engine and do not justify another gate.

- Rust: **5393 passed, 0 failed, 0 ignored**; **21 sparse tests explicitly ok**.
- Clippy `-D warnings`: finished clean in **26.25 s**.
- Native release compilation: **59.02 s**, no atomic deprecation warning;
  final Ninja line `[4/4] Linking CXX shared library FreeCity4.dll`.
- **Native terminal RC, complete qbuild all exit UNKNOWN; CTest unrun**.
  This is not a stable all PASS.

The live all is transport session **14747**. Exact chain at 04:00:10.526017,
04:02:22.635292 and 04:05:55.919555 UTC:

| Process | PID | Creation epoch | Child |
|---|---|---|---|
| qbuild all | 13496 | 1792727742.922702 | 29412 |
| fc4verify build ctest | 29412 | 1792727848.9993417 | 4940 |
| sccache --show-stats --stats-format json | 4940 | 1792727912.8739836 | none |

First three cwd values are the worktree. Own port 4713 listener remains PID
28648 creation 1792727063.2474122, cwd C:/Users/tlori/.cargo/bin, argv sccache.EXE,
conhost child 37924 creation 1792727063.2523866. No compiler descendants.
Exact argv/children/creation/cwd/scoped open_files/port/log bytes/hash/mtime/
source/time proof: `build-runs/r2-stall-all-{first,second,third}.json`; root's
case-specific request: `coordinator-request-core-rz-sparsematrix4-r2-all.md`.
Native log is unchanged at **989 bytes**. Targeted metadata had no AccessDenied;
open_files reported only MUI files, not exhaustive stdout/OS-handle ownership.
Discovery's Codex cwd AccessDenied is distinct from the observed checker chain.
Canonical audit `r2-all-terminal-review-audit.json` is a review label, not a
terminal result. Same-handle <=60-second collections have no terminal yet.
Engine tree remains 57061af0 with empty engine diff. No kill/restart/RPC or
additional checker was launched. Preserve the live checker for root decision.

### Evidence and blocker

Before fixed-log overwrite, archived seven original logs/request-note files,
**357186 bytes**, with verified SHA-256/byte manifest at
`build-runs/r2-original-evidence/manifest.json`. R1 fmt success and two earlier
relative-path/stdin-setting failures remain historical. No durable old terminal
was found among own build-runs terminal/check records.

OLD baseline: `22b7bff7`, engine `c30f4d9ac1d78880e81cae198b7a461307350c58`;
Rust 5372/0 across 23 summaries, clippy clean. Native log 217/217 is not a
terminal RC; final exit UNKNOWN, CTest unmeasured. These results predate the
kernel. Copied coordinator 04:37:56 BST proof found old 32304/24192/21872/6664
absent and no own references/4713 listener. Root did not terminate them;
disappearance establishes neither cause, cancellation nor success. Global
inaccessible system processes remain unknown. Do not wait for absent 83439.

NEW quick transport session **49758**, observed 03:49:18.992089 and
03:51:09.407676 UTC, with unchanged native log and no compiler descendants:

| Process | PID | Creation epoch | Child |
|---|---|---|---|
| qbuild quick | 29848 | 1792727060.8525696 | 34652 |
| fc4verify quick | 34652 | 1792727060.8896973 | 33108 |
| sccache --show-stats --stats-format json | 33108 | 1792727226.5452335 | none |
| own listener, port 4713 | 28648 | 1792727063.2474122 | conhost 37924 |

First three cwd values are the worktree; listener cwd C:/Users/tlori/.cargo/bin.
Conhost creation 1792727063.2523866. Exact argv/cwd/creation/children/scoped
open_files/port/log hashes/bytes/mtime/source identities are recorded in
`build-runs/r2-stall-first.json` and `r2-stall-second.json`; case-specific root
request: `coordinator-request-core-rz-sparsematrix4-r2.md`. Targeted snapshots
had no AccessDenied, but open_files reported only MUI files: no exhaustive
stdout/OS-handle ownership claim. Intermediate measurements also retained.

Quick Rust log: 238871 bytes, SHA-256
`14738db9fa8ce29164f9443d1840ccba8ccc4af3f6e4e18f942dd99700b1a023`.
Native log: 1702 bytes, mtime 1792727224.2836785, SHA-256
`c583da208d40cd4881442788d2de874c6ab3f863777f4f51193b33e9de8274aa`.
These historical quick logs are archived as four files/**244746 bytes** in
`r2-first-quick-evidence/manifest.json`. The quick eventually returned PASS/0;
its earlier request note is historical. Current qbuild-test/clippy/native fixed
logs belong to canonical all, not baseline/quick. No process cancellation/restart,
cache RPC, duplicate quick/all or old-baseline rerun. R2 q.py diagnosis failed
Bash E_ACCESSDENIED once; continued with PowerShell/Python. Commits succeed
despite nonfatal shared packed-refs.lock permission diagnostics; no attempt to
change shared refs/config was made.

### Per-entry ledger

These bounded effects passed in first quick on `f4bb6e0d` and workspace tests
on corrected engine `57061af0`. Literal Rust tests are not native or whole-owner
measurements; the full corrected-source gate remains incomplete.

| Entry | Delivery and qualification |
|---|---|
| 0095a0d9 | Partial lifetime wrapper omitted, SM3 |
| 009a5bf2, 009a5c36 | Foreign/misattributed record initializers omitted |
| 009a7577 | Identity equality SM-V11 passed; portable bool |
| 009a9122 | Column begin SM-V3 passed |
| 009a9225 | Maximum/sentinel tie SM-V9/V10 passed |
| 009a9280 | Signed inclusive count SM-V1 passed |
| 009a92db | Ordered 12-byte value range SM-V2 passed |
| 009a9a89 | Column extrema/absence/alias passed |
| 009a9ada | Row extrema/absence/alias passed |
| 009a9b7d, 009a9b9e | Row/column occupancy SM-V1 passed |
| 009a9bd7 | E borrowed payload lookup/miss preservation passed; pointer native alias SM5 unproved |
| 009ab6b4 | Partial: normal output/erase/count SM-V12 passed; foreign destruction/errors unresolved |
| 009ab716 | Partial: wrapped row removal SM-V6/V7 passed; foreign eraser errors unresolved |
| 009ab75e | Partial: wrapped column/successor SM-V8 passed; foreign eraser errors unresolved |
| 009ac02f | Partial: owned ten-word copy/order SM-V13 passed; foreign construction/copy/errors unresolved |
| 009ac157 | Partial: owned one-word insertion/bounds/order SM-V4/V5 passed; foreign construction/copy/errors unresolved |
| 009a8fec | Borrowed increment/holes SM-V3 passed; end unsupported, lifetime SM4 |
| 0095a087, 0095a0aa | Foreign AutoPtr release/destructor omitted |
| 0094c8f5, 009a947f, 009a94a6 | Shared lookup; maps supply storage, core owns selection/membership |
| 0040d4c0, 0040d500 | Foreign successor/predecessor layout omitted |
| 009abc11, 009599a9, 009a94ca, 009abb8d, 009ab2b6, 009a9c16, 009a9427 | Partial foreign dependencies; no failure/lifetime guarantees |
| 009a9c2f | Shared construction; no native defaults inferred |

SM1–5 remain open: attribution, foreign allocation/eraser exceptions, native
constructor/default/resize/copy/clear/ownership, invalid iterator/mutation
guarantees and unproved specialization aliases. Public 39/3358 attribution
remains UNRECONCILED. Windows 18 matrix entries/1784 bytes + iterator/82 + two
foreign entries/82 = 21 named entries/1948 bytes is not a whole-owner denominator.
No factory/default/native-complete/coverage promotion. No new question needed:
all excluded behavior is already covered by SM1–5.

### Next steps

Root must decide on the exact canonical-all stats chain, especially PID 4940.
Preserve it and collect SAME all session 14747 with waits <=60 seconds; old
disappearance does not authorize cancellation. The quick is complete; do not
recollect absent R1 session 83439 or launch another quick/all. Collect actual
all/native exit and CTest on unchanged engine 57061af0 and preserve stderr/log
hashes. Source errors justify bounded fixes/rechecks; report-only commits do
not justify another gate. READY requires the stable all to finish successfully.

Game/shadow n/a: standalone core, no request or native exactness claim. ABI
unchanged. Queue WIP. At most one further original round remains.

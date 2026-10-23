| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-rz-sparsematrix4: bounded sparse matrix, original R2 | Codex; core-rz-sparsematrix4 | in progress: quick terminal PASS/0, 21/21 literal tests passed; correction formatted | Run canonical all on stable corrected engine 57061af0 |

## Handoff

- Quick SAME session 49758 returned terminal exit 0: fc4verify quick PASS, 3349/0, 21 sparse tests ok, dependencies compiled, native build 4/4; fc4verify 474.6 s, qbuild PASS.
- Terminal explicitly says tree changed during run: atomic correction committed after compilation. Thus quick validates f4bb6e0d effects, not stable corrected engine 57061af0d159dc2c7803383c212fb04e8ca0c300.
- Durable terminal build-runs/r2-quick-terminal.json; first-quick archived 4 files/244746 bytes, manifest build-runs/r2-first-quick-evidence/manifest.json. Original evidence remains archived separately.
- The new stats wait is no longer a live blocker; its completion cause is unknown, no lane process action. Prior snapshots/report below describe observations at their stated times.
- Next exact step: one canonical qbuild all on stable committed corrected engine; capture true terminal and before/after source/log proof. No second quick required.
- SM1–5 stay open, no game/ABI change. Queue WIP until stable all PASS.

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
That correction is formatted but **not yet executed**. No stable all gate or
native success is claimed; this delivery must stay WIP.

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
- Native log reached `[4/4] Linking CXX shared library FreeCity4.dll`.
  **Native terminal RC and complete quick exit UNKNOWN**. Quick does not request
  CTest. Link progress is not a gate pass.

Actual warning: AtomicU64::fetch_update is deprecated on installed Rust;
try_update is newer than workspace Rust 1.75. `4c22040f` uses an equivalent
relaxed compare_exchange_weak loop with checked increment, preserving opaque
identity allocation/exhaustion. No algorithm or test expectation changed.
Both own Rust files pass qbuild fmt exit 0 after correction. The corrected
source still needs execution; new-source clippy and canonical all are unrun.
No duplicate quick/all was launched while the sole quick remains active.

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
Baseline qbuild-test/clippy filenames still contain old results. Same-handle
60-second collections returned no terminal. No process cancellation/restart,
cache RPC, duplicate quick/all or old-baseline rerun. R2 q.py diagnosis failed
Bash E_ACCESSDENIED once; continued with PowerShell/Python. Commits succeed
despite nonfatal shared packed-refs.lock permission diagnostics; no attempt to
change shared refs/config was made.

### Per-entry ledger

These bounded effects passed on `f4bb6e0d`; corrected engine `57061af0` still
needs execution. Literal Rust tests are not native or whole-owner measurements.

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

Root must decide on this exact new owned stats chain. Preserve it and collect
SAME quick session 49758 with waits <=60 seconds; old disappearance does not
authorize cancellation. After true terminal, execute the committed correction
and run one canonical qbuild all on stable committed source. Preserve stderr,
exit/log evidence, actual Rust/clippy/native RC/CTest and before/after engine
hash/tree. Source errors justify bounded fixes/rechecks; report-only commits
do not justify another full gate. READY requires a stable all pass.

Game/shadow n/a: standalone core, no request or native exactness claim. ABI
unchanged. Queue WIP. At most one further original round remains.

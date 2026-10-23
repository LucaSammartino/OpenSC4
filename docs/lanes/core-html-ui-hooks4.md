| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-html-ui-hooks4: standalone HTML hook algorithms | `core-html-ui-hooks4` (engine lane, detached HEAD) | READY R3 FINAL3/3: canonical all exit 0; cargo 5380/0, clippy 0 diagnostics, native PASS 2/2 edges, ctest 68/68; HTML fixtures 8/8; engine 2abc42af and content unchanged; ABI unchanged; game n/a; HU1-HU4 open | 1. Coordinator may land bounded standalone implementation. 2. Foreign domains and native/game integration require their own contracts and qualification. |

## R3 final report: actual stable full qualification

This is the original FINAL3/3, completing initial qualification of the new
standalone implementation. Resumed clean `02319214`; milestone `c72fe0b6`
records the qualification handoff. R3 changes only this report. The implementation
and independently authored expected ledgers were unchanged throughout the run.

### Delivered bounded domain

`engine/core/src/html_ui_hooks.rs` owns classification and document selection,
resource key construction, segment acquisition/read/release order, supported
link dispatch decisions and AL result forwarding, logical shared references,
QI identity and lifetime ordering. Primitive seams retain raw opaque resource,
URL, lifetime and window operations; owned decisions and ordering remain in core.
`html_ui_hooks_tests.rs` has eight independent authored tests: the published
14/9/7/6 trace groups and four boundary/integration fixtures. All eight executed
successfully in this final workspace run, as well as the historical R2 quick.

This qualifies the existing bounded Rust algorithms. ASCII/valid-hex or supplied
conversion results, logical tokens/snapshots and the explicit 16 MiB resource
allocation bound retain their documented limits. General URL normalization,
foreign conversion/service semantics, invalid-memory/allocation/exception
domains, registration retention and reentrancy remain HU1-HU4 boundaries.
Unsupported domains do not acquire invented native false/cleanup semantics.
Published attribution is nine entries / 1272 bytes, with no native-byte or
runtime game coverage credit. No App/window integration, CAPI/plugin/ABI/default,
CMake, registration, algorithm expansion or other-owner changes were made.

### Actual final gate and terminal

Exactly one canonical command was launched after preserving old fixed logs:
`python C:/Users/tlori/FreeCity4/.claude/lane-bin/qbuild.py --wt C:/Users/tlori/FreeCity4/.claude/worktrees/core-html-ui-hooks4 all`.
The local evidence wrapper only preserved logs/source and observed the owned
process subtree. No new baseline, quick, focused rerun, cache configuration/RPC,
process cancellation, duplicate checker, kill or intervention occurred.

- Workspace cargo: **5380 passed, 0 failed, 0 ignored**, summed across all
  **23** `test result:` lines; command exit **0**. All **8/8** HTML tests are `ok`.
- Clippy with `-D warnings`: exit **0**, **0 diagnostics**, finished in **27.83s**.
- Native build: **PASS**, **2/2 edges**, **0.5s**. This edge count is separate
  from the actual fc4verify exit and CTest result.
- Full CTest: **68/68 passed**, **0 failed**, reported **76.6s** (log **76.53s**).
- fc4verify: **PASS**, **79.4s**; qbuild: **PASS**; actual terminal exit **0**.
- SAME yielded exec session **44983** was collected to actual terminal; the
  evidence wrapper also exited **0**. No stalled optional observer remained.

Exact qbuild identity: PID **15180**, creation **1792728361.6747768**, parent
**28956**; cwd is this worktree. Exact Python executable and argv are retained
in `build-runs/html-r3/qbuild-process.json` and `invocation.json`. Owned
fc4verify PID **28768**, creation **1792728471.3354423**, parent **15180**,
and CTest PID **25480** are recorded in `observations.json` with exact argv,
cwd, creation, parent and port (CTest inherited **4466**). Scoped snapshots had
no observation errors. No global spawn/CIM/Git/npm preflight was repeated.

### Source and log proof

The actual command ran on committed HEAD
`c72fe0b653442897ffbe94bab684bcb7113feee3`, tree
`3e1e2aaabcb029b8e854b5a4793289e95df72f4c`. Engine tree before and after:
`2abc42af3645dd5e7c1f5c72fad9f0af0a0955e4`. Aggregate tracked regular engine
file content SHA256 before and after:
`d062d7ff47524183a89b56b119859843f57295cde43d92adf4288a59b0c73dfe`.
Both source snapshots had clean status; `terminal.json` records actual rc 0,
`same_engine=true` and `same_content=true`. Exact per-file hashes, including
the unchanged R1 HTML hashes below, are in `source-before.json` and
`source-after.json`. The final report commit changes Git tree/HEAD only; it
does not require a cosmetic qualification rerun on identical engine content.

Current fixed logs under `build-runs/`, SHA256:

- `qbuild-test.log`: `e31ab609cacda9b3ff06ccdcd1be56224593a8e88b06808da912c2b3c8a69d02`
- `qbuild-clippy.log`: `c0b22ace050dc71c21bc08ac7da809e9555e81bf4dc9374510a63831d5453f34`
- `fc4verify-build.log`: `e052e424cf5b849036087d5067c3085b9b70df2d295f7582f9d16511ef3095df`
- `fc4verify-ctest.log`: `65ba5a57288a79f5a1a31406c374500586537618e461280f70732e284c7010c9`

`html-r3/terminal.json` includes all fixed-log paths, sizes, hashes and mtimes.
`html-r3/preserved-logs.json` and `pre-r3-*` preserve the old fixed logs before
the canonical all. Old quick logs were unchanged and supply only R2 credit.

### Historical limits and disposition

The copied 04:59 preflight found old 23048/25300/35180/9568/36088/34912/15368
absent, matches/errors empty, daemon 8644 with only conhost on the supplied
4466 proof. Old HTML quick terminal and disappearance cause remain **UNKNOWN**.
Root's 04:52 finite action affected Catalog/Sparse/Registry only: **no root
HTML kill**. Old absence is not a successful gate. Historical R1 source baseline,
formatter/path/header/check difficulties and R2 actual core 3336/0 plus eight
fixtures remain historical evidence, retained below; this final all supplies
the first stable full qualification of the current engine. No independent
otherSC4classes enginecheck or native game/shadow result is claimed.

New questions: **none**; existing **HU1-HU4** remain open in
`docs/specs/questions/html-ui-hooks.md`. Game runs/shadow/ref_warnings: **n/a**
for this standalone no-game lane (`game=false` in the manifest). ABI: unchanged.
READY applies to the bounded implementation with this actual final gate.
The own merge queue uses the actual final detached HEAD; the supervisor owns
publishing the reserved branch. No fourth round or readiness override is used.

## R2 historical report: then-current qualification and blocker

This is the original lane's second round. Source and all eight authored fixtures
are unchanged. No evidenced compiler/test/published-contract defect justified a
source change. The 36 independent groups, HU1–HU4, raw primitive boundaries and
9/1272 attribution without native-byte credit remain as reported in R1 below.
No App/window/CAPI/plugin/ABI/default/coverage or native/game expansion occurred.

Milestones: `bcf0332e` records R2 liveness and historical limits; `fd3411e4`
records actual fixture execution and the exact new observer blocker. Both change
only this lane report. No dirty work was discarded or reformatted.

### Operation ownership and historical baseline

The first operation inspection found old qbuild 35852, fc4verify 8132, stats
34624 and backend 3324 absent. Scoped psutil preflight independently found no
own checker/compiler hits. Global AccessDenied is explicit in
`build-runs/html-r2/preflight.json`; this is not an exhaustive host assertion.
Daemon 8644 creation 1792726273.2470572 and conhost 20024 creation
1792726273.2513285 matched the supplied port-4466 proof. Daemon identity differs
from earlier 22580; disappearance/replacement cause is UNKNOWN.

Read the exact copied `coordinator-six-stats-observer-interruptions.json`:
root interrupted ONLY Grid 29472 and Catalog 37168 (each exit 15). HTML stats
34624 and its parent were already absent; root took NO action on HTML. Its
baseline exit/native RC and disappearance cause remain UNKNOWN. Session 92092
is not an available handle in this CLI, and no cross-CLI wait was invented.
Existing baseline logs were preserved under `build-runs/html-r2/pre-r2-*` with
hashes/mtimes in preflight.json. Their Rust/clippy/build hashes match R1.
Pre-existing ctest artifacts have later mtimes and unqualified source/provenance;
they are preserved but supply no R1 terminal/native RC or R2 ctest credit.

### Actual current-source focused run

Started **one** canonical `qbuild quick` after the actual no-checker inspection,
at HEAD `bcf0332e4885adf451a60d9a70d7458bb3ff1efe`, tracked engine tree
`2abc42af3645dd5e7c1f5c72fad9f0af0a0955e4`. It tested fc4-core on i686 MSVC:
**3336 passed, 0 failed, 0 ignored**, plus a zero-test doc result. All **8/8**
HTML fixtures appear by name as `ok`, including the 14/9/7/6 group fixtures and
four boundary/integration fixtures. Test compile 53.66s; unit execution 2.92s.
The expected ledgers were not changed to fit these results.

`fc4verify-quick-test.log` SHA256:
`a01adaaf0ab0c9e55b0d242e058304356aded8cfdb2bf267fe4b567656712cc2`.
Native log ends `[4/4] Linking CXX shared library FreeCity4.dll`; that is an
edge count, **not native RC or a terminal gate verdict**. Its SHA256 is
`7f825c79c850a807b4b26b0b0307cdeb8f818317ef2d45a44aaeb92aff3212cc`,
mtime 1792727253.2905436. Source file hashes remain exactly the R1 values below.

### Active blocker and exact next action

The SAME quick validator remains alive, waiting on its read-only optional cache
statistics client. Terminal collection in the same exec session returned no
output; no exit was invented. Session **45516** is still the owned handle.
Final scoped snapshot: `build-runs/html-r2/final-ownership.json` (295 global
AccessDenied entries retained; no own compiler hit). Exact identities:

- qbuild **36088**, creation **1792727089.2347448**, parent 9568.
- fc4verify **34912**, creation **1792727089.2772796**, parent 36088.
- stats **15368**, creation **1792727255.100597**, parent 34912;
  `sccache.EXE --show-stats --stats-format json`, `SCCACHE_SERVER_PORT=4466`.
- All three cwd is this worktree; exact argv/environment and log identities are
  in `quick-observation.json` and `final-ownership.json`.

One finite coordinator review request is recorded at
`build-runs/coordinator-request-core-html-ui-hooks4-r2.md`. Coordinator owns
review/intervention eligibility; lane has made no RPC, kill, cancel, signal,
daemon restart or cache config change. Revalidate exact identities before any
root action, journal it, and collect **this same validator** to terminal. Do not
launch another quick/baseline/cargo/ctest while it is live. Then run **one**
canonical `qbuild all` against stable committed source, retaining terminal exit,
true all-Rust sums, clippy, native RC, ctest and before/after engine identities.
Current-source full gate and clippy remain **unmeasured**, and all was withheld
because quick is still active. No new baseline was run.

Supplied real host proof `coordinator-html-hooks-r2-host-proof.json` records
full `other SC4 classes` engine `--resume` lane check **exit 0**, stdout `ok`,
dependency-ancestor-main **exit 0**. Current manifest/runtime guard is valid.
This is distinct from an enginecheck capability: none is configured here;
no invented capability, substitute claim or tools/ execution was used.
Historical R1 failures below remain retained. No formatter was rerun because no
Rust file changed. No new questions; existing HU1–HU4 remain open. Game: n/a,
no requests, native/shadow/default/whole-owner credit. State remains **WIP**.

R2 manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-html-ui-hooks4-r2-manifest.json`.

## R1 historical report (retained; present-tense check status is historical)

## Delivery and limits

Standalone `engine/core/src/html_ui_hooks.rs`, independent primitive fixtures in
`html_ui_hooks_tests.rs`, and one additive export at the end of `lib.rs`.
No window integration, App wiring, plugin, CAPI, ABI, default or coverage edits.
No game request, script execution, browser, filesystem/network loader or command
launch. No shadow/native conformance or whole-owner promotion is claimed.

Commit milestones: `30b9bc53` scope/baseline; `0b8a416e` independent literal §7
expectations **before** production; `79856cad` first algorithms; `8ffe72d2`
remaining boundary fixtures and separated raw city/advisor primitives.

Owned algorithms include ASCII scheme equality, exact lowercase full-URL gate,
literal slash token ownership (including no-separator source retention), label
mnemonics vs explicit group, 32-bit key stores, >=8 instance conversion routing,
retained-service/key acquisition, first EAX size overwrite and exact FFFFFFFF
sentinel, prefix-preserving resize, ignored second result/size, and segment
Release on supported true/sentinel paths. Failed acquisition never Releases,
even with a non-null segment output. Dispatch owns first-occurrence gates,
action local token processing/destruction/false, raw advisor retrieval and whole
payload forwarding with AL extraction, without executing commands.

Logical allocation/base/hook views model shared QI identity, count-zero start,
wrapping AddRef, Release count 0/1 destruction, separate RemoveRef, Init's
AddRef/store/old-Release/temp-Release order ignoring lookup AL, equal-pointer
cleanup, Shutdown clear-before-Release and destructor uncleared Release followed
by base destruction and low-bit free. Tokens are not native pointers or ABI
alignment claims. Callbacks see immutable snapshots and cannot mutate the hook.

HU1–4 remain open in `docs/specs/questions/html-ui-hooks.md`; no new questions
filed. ASCII/valid hex only, except explicitly supplied raw conversion results.
No general URL parsing, percent/dot/query/fragment normalization or inferred
malformed-hex semantics. Unsupported is distinct from native false: explicit
16 MiB resource allocation policy, null service, success-null segment, missing
city/advisor, non-ASCII/embedded NUL, unspecified hex and destroyed-object
operations. Allocation/invalid-memory/exceptions, actual foreign service
semantics, registration retention and reentry remain unsupported. Unsupported
resource allocations stop a partial trace, without inventing native cleanup.

## Per-body accounting

All rows below are **bounded implemented, runtime effect tests pending**.
Fixtures are authored conditional expectations, not executed native coverage.

| Published body | Owned bounded implementation | Foreign / unknown |
|---|---|---|
| 006581F0, 8 bytes | Logical hook/base identity shares Release | Native pointer subtraction/ABI layout |
| 006626F0, 5 bytes | Release forwarding to shared logical count | Native thunk/code bytes |
| 0078CF80, 49 bytes | QI 1/C49B192C same hook view, failed output unchanged, AddRef | Invalid output pointer domains |
| 0078CFC0, 97 bytes | Init replacement/temporary order and unconditional true | Lookup/service guarantees, HU4 |
| 0078D030, 24 bytes | Shutdown clear-before-release, repeat safe logical call | Reentrant/mutating callback behavior |
| 0078D0C0, 114 bytes | ASCII GetScheme equality only | URL extraction, non-ASCII HU1 |
| 0078D2B0, 115 bytes | Two-phase read choreography, sentinel/resize/return | Real database/read validity, HU2/HU3 |
| 0078D330, 492 bytes | Prefix/token/key stores, converter inputs, retained reader call | URL codec/hex outside supported domain, HU1/HU3 |
| 0078D140, 368 bytes | Action/advice gates, locals and dispatch AL | Advisor effects, invalid city/advisor memory |

The first eight attribution entries total 904 bytes; DoURL adds 368 (9/1272).
These are **attribution inputs, zero whole-native byte credit**, not proof every
instruction/foreign primitive is implemented. Supporting constructor 0078D050
(39), destructor 0078D080 (57) and shared reference routines are not additional
original-nine coverage; no double counting. Logical construction/destruction
does not claim physical table writes, native allocation or registration.

## Actual checks and blockers

Initial required `qbuild all` launched on initial detached `5e49ebfe` (engine
tree `c30f4d9ac1d78880e81cae198b7a461307350c58`). Its Rust log contains **5372
passed, 0 failed, 0 ignored across 23 result lines**. Clippy completed in 26.11s.
Native log reaches [217/217] link, but **native RC and ctest are unavailable**:
fc4verify is blocked in its stats RPC before ctest. Baseline terminal has not
been returned (exec session **92092**). None of those initial results validates
the new source. No quick/second all/cargo/ctest or cache RPC was launched.

Exact intervention request is `build-runs/coordinator-request-core-html-ui-hooks4.md`.
Scoped psutil observation: qbuild PID **35852**, creation **1792725156.09832**;
fc4verify PID **8132**, creation **1792725269.4405117**; its stats child PID
**34624**, creation **1792725370.8602865**. All cwd is this worktree; child argv
is `C:\Users\tlori\.cargo\bin\sccache.EXE --show-stats --stats-format json`.
Full parent argv/cwd and log locations are recorded in that local request.
No process was killed, cancelled, restarted or signalled. Revalidate identities
before any coordinator action; snapshot does not grant process authority.

Focused fixtures comprise the 14 classification/load, 9 dispatch, 7 resource
and 6 lifecycle **groups**, plus boundary tests: growth/shrink fills, huge sizes,
valid/supplied conversion domains, loader integration and zero/wrapping/shared
QI/RemoveRef behavior. Eight Rust tests authored; **not executed**. Both new
files pass quiet-wrapper fmt (exit 0); `git diff --check` passes. Full mandatory
`otherSC4classes` enginecheck is unavailable: no configured tool matched
enginecheck. No narrowed substitute or claim was made.

Failures retained: initial process WMI query denied; formatter helper initially
failed on Windows stdin encoding and then a mismatched-encoding patch (no
partial patch applied); relative-path fmt could not find files; first helper
format used width 100 instead of engine's width 120 and was corrected through
apply_patch. Absolute touched-file fmt including lib reported existing module
ordering differences (188 lines), which were deliberately not mass reformatted.
New files subsequently pass with engine config. Git commits print a shared
packed-refs.lock permission diagnostic but return exit 0 and create the recorded
detached commits; no ref/config/hook modification was attempted. The baseline
stats stall and unavailable native/ctest results remain unresolved failures of
the required validation, not successful gate results.

## Source and evidence identity

Committed source `8ffe72d208d254495b69419ae32864c45cd73051`, engine tree
`2abc42af3645dd5e7c1f5c72fad9f0af0a0955e4`. Final report commit changes only
this report; engine tree must remain identical. SHA-256 of actual files/logs:

- html_ui_hooks.rs: `32a225ae4948191bfc532ba8216a25b5e6b8b44958c0b18423fd219eae9e0e5b`
- html_ui_hooks_tests.rs: `93ea9a7fc8f1fde596994949bf99e6cc90b9f339f9cdd0ecfb02ea84460fbbf6`
- lib.rs: `cb3f0375b8bac9082a17e24fc35d01763fd99e89429b59431305bb20e80c7254`
- qbuild-test.log: `ac7749e7dbc252277853ec19ffa6ea1d2da7a936de1820974e4e210fea66b1f4`
- qbuild-clippy.log: `680dc8a54f05c0177c45db59ba4ac82d12fe7022e3271bbd4d4ee05e89e62e7a`
- fc4verify-build.log: `aec4a55bdfb3e6f1cb68d861e40c1bf04403b67f5dfe96e938afa2e932211e6d`

## Next steps

Coordinator resolves the exact baseline stats-client stall under its existing
process policy; collect the **same** session 92092 terminal and preserve its
failure/RC/ctest evidence. Then run focused `qbuild quick` for current source,
fix actual failures, commit stable engine source, and run **one** canonical
`qbuild all`, collecting the same yielded session through terminal completion.
Obtain full otherSC4classes enginecheck. Report actual current-source Rust,
clippy, native RC, ctest and source hashes before changing WIP to ready. No game
request is appropriate for this explicitly standalone/no-game scope. Manifest:
`C:/Users/tlori/FreeCity4/build-runs/run4-core-html-ui-hooks4-r1-manifest.json`.

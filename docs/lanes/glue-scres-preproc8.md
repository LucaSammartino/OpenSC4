| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-scres-preproc8 physical preprocessing | Codex / glue-scres-preproc8 (supervisor-reserved detached HEAD) | READY: SCR11 vectors16/16; Rust6936, clippy0, CTest74/74; raw scres149/149 and default scres76/76 clean to2032 | Coordinator: append scres_file_shadow to current single house shadows string; review scoped coverage credit |

## Report

The full raw physical-file replay goal is met. `f90a7edfc` implements SCR11
in the production `parse_file_with` loop used by FileParser and passive replay.
The exact first `#<` erases its tail before the active flag searches the retained
prefix for the first `#>`. Only the closing suffix is routed; quotes, groups,
ordinary comments, repeated openers and same-line marker order retain the spec
quirks. Each recursive/registered file has its own flag, all true physical reads
count, suppressed includes never open, and active EOF succeeds. All sixteen
SCR-V93–V108 routing vectors are executable tests, including child state/counts.

The temporary foreign-preprocessing control replay was removed. Raw replay
receives the physical bytes and computes suppression through production code;
there is no native suppression input, seeking or resynchronization. Ordinary
command execution remains an observed foreign boundary. The ini key
`scres_file_shadow` remains default-off. No public C ABI export or version changed.

The requested branch merge is `866a2a718`. Git emitted a shell/mapping failure
and committed a tree missing replay and SCR11 artifacts; `757072981` restores
only those artifacts, preserving current ABI93 and other lanes configuration.
The open questions file already omitted SCR11; its answer is present in spec
section12 and the answered archive. No new question was needed. The predecessor
report is restored; its old run records remain available on the merged replay7
branch because their IDs403–406 overlap current-main run records.

## Gate

- Focused tests: SCR11 core vectors **16/16**, capi raw replay **3/3**.
- Owned Rust-file formatting **PASS**, zero diffs; fixes applied with apply_patch.
- One final stable-source `qbuild all`: **6936 passed, 0 failed, 0 ignored**;
  clippy `-D warnings` **0 diagnostics**; native build **240/240** (93.5s),
  CTest **74/74** (143.2s), native total240.4s, terminal **PASS**.
- Native steps used the coordinator verification queue, request
  `qbuild-20261024-052626-build-ctest`, collected once to its actual result.
  No direct MSVC configure/build or second queued verification was launched.
- The verifier tree-change note concerns documentation checkpoints only:
  `git diff f90a7edfc HEAD -- engine` is empty. Both games used this source.
- Full logs: `build-runs/qbuild-{test,clippy}.log`,
  `build-runs/fc4verify-{build,ctest}.log`. The initial shell-wrapper failure
  before focused tests is preserved at `build-runs/q/20261024-051910-28568.log`;
  direct Cargo succeeded. No failed gate was replaced or mislabeled.

## Measured runs

Both unique requests reached2032 in Konradshohe, **clean**, **ref_warnings=0**,
all harness problem counts0 and no resynced lines. Each used one event-driven
blocking wait without qwait; no request or verdict was discarded.

| Run record / label | Exact counted summary | Scope |
|---|---|---|
| [449](../runs/scres8-raw-2032-maple-437.md) / `scres8-raw-2032-maple-437` | **scres 149/149** | raw73/73 plus kept76/76; clean to2032-01-05 |
| [450](../runs/scres8-default-2032-oak-436.md) / `scres8-default-2032-oak-436` | **scres 76/76** | new key omitted; no raw replay lines; clean to2032-01-16 |

Run449 has **73 dated raw_file_parser_replay lines**, from2026-01-27 load
through2032-01-01 month, every one **mismatches=0, resyncs=0, refused=0**.
Each reports roots2/replays2, reads1107, blank122, false EOF-or-failure2,
opens3/open_failed1, commands1061, includes1, physical1107,
inclusive_totals1530, undispatched45. Existing native tokens remain842/842;
constructed binary24/24, text22/22 and resource-prefix10/10 are exact.
There is no remaining turbo-only difference requiring a normal-speed rerun.

DLL SHA256 for both runs:
`0f40f2a589fad1fe2ca25f3389b52974f723ad88a593044c957c4472a3f45a94`.

## Coverage and coordinator next steps

The newly exact contract is **nSCRes::cFileParser DoParseFile** routing,
physical/inclusive counts and SCR11 preprocessing (sections4.2,11,12) on the
natural startup file tree, including optional open failure. This wholly verifies
that observed raw file-loop replay contract. It does **not** wholly verify a
broader owner row including unobserved methods. Specifically:

| Owner row | Whole-owner credit from this lane |
|---|---|
| nSCRes::cFileParser | No: lifecycle, watcher/message callbacks, modified/error/parser getters and all registered-file hook outcomes are not differentially exercised |
| nSCRes::cCommandParser | No: token inputs are exact; command execution, registry/macros/variables and state are not replayed |
| cRZCmdLine | No: framework command line is untouched and unmeasured by this shadow |
| Other scres token/adapter owners | No new whole-owner claim; previous adapter-read measurements are preserved |

Thus **no additional whole-owner coverage-map row** is claimed. Credit the
exact file-loop slice without broadening it to command execution or ReadLine
byte decoding. Coverage map/shadow files were not opened or edited.

Now that raw replay is exact, the coordinator should merge against its current
main and append `scres_file_shadow` to the **existing single** house-profile
`shadows = ...` string, then run its house regression. This lane did not access
`tools/regress.ps1`: the task explicitly forbids opening/reading any tools path
under its non-bending clean-room rules. No permission was requested or inferred.
There are no remaining spec gaps for the implemented raw loop, no questions
filed, and no build/game operation outstanding. Ready queue uses actual detached
HEAD; supervisor publishes the reserved lane branch. No push/ref update made.

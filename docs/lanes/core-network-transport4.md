| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-network-transport4: portable socket/stream owner | Codex / `core-network-transport4` (engine lane) | ready: first stable canonical all PASS, original round 2/3 | Normal own queue; coordinator publishes detached tip; Q11-14 remain open |

## Final qualification, original round 2/3

The already-authored production owner and its tests are unchanged. Production commit
`58dcdb7d3d5c99f99f4a2bf021716341cf655dfc`, starting clean report tip
`0ae3391358a11908a8f5a6efce89b3366c9713b0`, and qualified report tip
`d2e45d149c9c8a57a6f95d5701bb70e1dcc37a44` share engine tree
`031bb4830a3a8ecc148148bc6a1d10068a503aa4`. No diagnostic required source repair.
This consumes original round 2 of 3, leaving one original round; no reset or alias.

Exactly ONE first stable canonical `python C:/Users/tlori/FreeCity4/.claude/lane-bin/qbuild.py
--wt C:/Users/tlori/FreeCity4/.claude/worktrees/core-network-transport4 all` completed
with actual terminal RC **0**. Exec session **93383** was collected through its terminal
using the same supported session. Checker PID **38340**, creation **1792735962.1957507**;
recorder PID38352, creation1792735962.0312989. Operation began **06:12:42.116566 UTC**,
ended **06:16:26.977765 UTC** on 2026-10-23; qualified HEAD/engine stayed unchanged.

- Workspace Rust: **5537 passed, 0 failed, 0 ignored**, summed over all **26** actual summaries.
  Passed counts in order: 540, 3, 1, 31, 27, 40, 7, 2, 1232, 3440, 21, 13, 19, 147,
  0, 2, 1, 3, 1, 1, 1, 5, 0, 0, 0, 0. Every summary is `ok`.
- Clippy `-D warnings`: **exit0, diagnostics0**, actual finished dev profile.
- Native build: **PASS**, 2/2 incremental edges, 0.6s; full native verifier terminal **PASS**,
  82.8s. This is build/test qualification, with no original-game/native vector execution claim.
- Full CTest: **68/68 passed, 0 failed**, 79.86s; wrapper returned **qbuild: PASS**.
- All owned SHA-256 values revalidated after completion; `git diff HEAD -- engine` empty.
  Final changes are documentation only. No repeated focused/baseline/quick/format check.

Durable evidence is under `build-runs/transport-r2-all/`: `preflight.json`, `operation.json`,
`session.json`, `terminal.json`, raw `terminal.log`, `qbuild-{test,clippy}.log`,
`fc4verify-{configure,build,ctest-list,ctest}.log`, and `summary.json` with every summary,
totals, owned hashes and raw-log hashes. Recorder and independent summary scripts are
`build-runs/transport-r2-{qualify,summary}.py`. Inherited `fc4verify-quick-*` logs remain
historical and supply no new full-gate credit.

Owned SHA-256:

- `engine/core/src/lib.rs`: `82a68730da82b6e9f63b653ad03ca8d7c736780567e681910642eebf6da910ed`.
- `engine/core/src/network_transport/mod.rs`: `8301fff8701405d8e9ecd5ceca51598aa41cb61532e39aa0595a0f95e47aaa52`.
- `engine/core/src/network_transport/tests.rs`: `e685c9552c25ada408df12f46593bbd4b006d57c06f1f13dd54e623d3d142a62`.

## Historical failures, intervention and drain

R1 quick session22211 has **UNKNOWN historical terminal/exit**. Its native log reached
217/217 link entries at06:45:25 BST, SHA256
`d7e8079fcfbe204015365a05654c6d88e756f3e4cfa1527767024fb3cafa6aed`;
these are not native terminal evidence. The q.py Bash startup **E_ACCESSDENIED** is a real
startup failure, not a compiler failure. Historical Rust3493/0 and owned fmt0 are retained.

R1 omitted an actual root intervention: watcher9 terminated **ONLY optional statistics
PID8008**, creation1792734327.1859694, **exit15 at06:53:23.406467 BST**. Exact journal:
`build-runs/coordinator-optional-stats-20261023-065321.json`. At the action, the same
verifier29196/creation1792734147.289136 and daemon18884/creation1792734149.748263
remained alive, source58dc/engine031bb, native_failure_terminal=false. No validator,
compiler, daemon or cache restart is established, and no gate credit follows from this action.
Later absence has **UNKNOWN cause/exit**. No R2 root statistics intervention was observed.

Copied authoritative HOST evidence records all historical family PIDs absent at07:04:49 BST;
census07:05:30 BST has own_matches/errors/services empty, port4617, known DLL/PDB/native-log
write-access probes clear. At R2 preparation06:11:41 UTC the exact historical
PIDs21424/38576/29196/8008/18884 were still **ABSENT**; owned files matched and HEAD was clean.
This establishes observed drain with copied HOST proof, not historical terminal proof or
universal host visibility. No stale-session waits, new global inventory, stats RPC, kill,
restart or configuration change was performed by this lane. Prior raw logs and copied
HOST/admission/journal evidence were preserved before overwrite in
`build-runs/transport-r2-all/historical-r1/`. The documentation commit printed a
packed-refs.lock permission error but committed successfully as detached d2e45d14;
no refs/config/maintenance workaround was attempted.

## Implementation and method/domain ledger

This is the first bounded socket/stream state owner found in the allowed production source. Existing `www::ftp` has external owner-level socket operations and a host/port pair; it was not changed. The new `network_transport` module exposes a logical isolated Rust API, with additive `lib.rs` wiring. No native layout, exported entry, QI/factory identity, original execution or body-byte coverage is claimed.

| Domain | Actual portable implementation | Limits |
|---|---|---|
| Construction and scalar fields | Socket/stream defaults; ID full argument; save low raw byte; style, created test | Rust zero address tails are explicitly qualified storage only; native constructor guarantees only first word |
| Addresses | Complete raw16 setters, bind/copy-before-call, local bind wrapper, explicit-handle and own-handle getname paths | No endian conversion; failed raw writes retained; native adapters return Unsupported under Q13 |
| Lifecycle | Unconditional create replacement, base/stream close, shutdown, listen/stop, handle and object accept | No old-handle close/rollback; object accept orders H/peer, state, style and ignores style bool through portable destination dispatch |
| Connect | Timeout-zero raw connect with timer construct/connect/error/destroy order; peer-taking wrapper | Nonzero timeout returns Unsupported before mutation; no scheduler or actual timer |
| Teardown | Explicit base close unless saved, opaque auxiliary cleanup/free/clear, owned target release/clear, optional object free, identity return | Separate from safe Rust Drop; no inferred allocator/resource kind, COM binding or final refcount |
| Transfers | Read/Peek/OOB/Write/OOB; exactly one raw recv/send, count updates, raw prefix writes, stale errors, EOF stream-close override | Negative non-Read extents and insufficient safe storage Unsupported; malformed raw counts Unsupported; no fill/retry loops |
| Readiness | Read/write asymmetric failure, unsigned timeout/null, combined mask and read-then-write memberships, waiting size/ioctl, connected select | No clock, select retry or close on readiness error; ref outputs retain sentinels where specified |
| Error and callback helpers | Full published u32 translation table; isolated raw captured-handle close callback | No OS error library or timer scheduling |
| Residuals | Notifications/options explicit Unsupported barriers; native address adapter barrier | NameLookup, startup, async messages, factories and native bindings are unimplemented; enum variants do not imply implementations |

## Measured validation and residual work

- 38 passing portable production-method fixture tests, authored with explicit expected literal values before production implementation: L1–11, I1–7, A1–3 (A3 raw record patterns plus Unsupported adapter), R1–7, plus distinguishing failure/ownership/dispatch cases. These are not executions of the spec's proposed native vectors.
- Historical focused Rust: core **3440 passed / 0 failed** (includes those 38); integration groups **21/0**, **13/0**, **19/0**, doc group **0/0**, aggregate **3493/0**. These are retained without repeating focused tests. The first full workspace gate above independently includes these groups.
- Owned-file in-place rustfmt followed by quiet `fmt` with absolute paths: **exit 0**. The wrapper's home canonicalization warning was nonfatal. Initial `q.py` attempt failed Bash startup with E_ACCESSDENIED; direct in-place rustfmt succeeded. No broad formatting or environment/config change.
- Historical quick native terminal remains **UNKNOWN**; its link log is not upgraded by the separate actual full gate above. No baseline all was run.
- Stable-source audit: HEAD `58dcdb7d3d5c99f99f4a2bf021716341cf655dfc`, engine tree `031bb4830a3a8ecc148148bc6a1d10068a503aa4`; per-file SHA-256 values are in `build-runs/transport-evidence.json`. Final documentation commits leave that engine tree unchanged.
- No game run or shadow: this module has no native/game adapter or switch. No ABI/CAPI/plugin/default/map/generated-coverage changes, no foreign owner changes, no native-count or broad web-family credit.
- Questions filed: **none**. Published Q11–14 and old web Q1/Q3/Q7/Q9/Q10 remain open. Storage/dispatch choices above are explicitly portable, not answers to those questions.
- **READY** for normal own merge queue after this report commit. Coordinator publishes the reserved detached tip. Remaining scope is explicitly unsupported as listed above and subject to open questions; historical quick exit remains unknown and is not a qualification prerequisite after the established drain and actual first full PASS.

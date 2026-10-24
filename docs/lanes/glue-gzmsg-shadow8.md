| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-gzmsg-shadow8 (queue manager shadow) | Codex / `glue-gzmsg-shadow8` | in progress | Inspect public sampling contracts; implement and measure available state. |

## Handoff
- Goal: default-off read-only gzmsg shadow of the queue manager, city load and monthly through 2032.
- Done: implemented default-off `gzmsg_shadow` load/month hooks and private Rust core lookup comparison; filed GZ11 for absent full-state observation contract.
- Scope: 259 explicit IDs (0..255, service ID, IID, UINT32_MAX), immutable numeric load baseline, monthly return/pointer equality. No native references retained; no resyncs; no whole-owner credit.
- Done: implementation committed as `488be884c`; focused tests 2/2 and explicit absolute-path fmt passed.
- Gate: `qbuild all` terminal PASS: Rust 6940 passed, 0 failed; clippy 0 diagnostics; coordinator native build 250/250 edges and CTest 78/78. Logs build-runs/qbuild-{test,clippy}.log and fc4verify-{build,ctest}.log. Native reported tree changed: only lane documentation changed during the gate, no source/input changes.
- Next: collect `gzmsg-shadow8-r1-2032` verdict, then count exact dated lines and inspect shutdown safety counters; write run record/report/ready queue for measured scope.
- Facts: ONE outstanding game request: `gzmsg-shadow8-r1-2032`, turbo Konradshohe to 2032-01-01, gzmsg_shadow=1, refaudit=1. Wait via build-runs/wait-game.ps1, never submit a duplicate; an existing verdict is valid.
- Facts: q.py sandbox Bash failure preserved; direct Cargo succeeded. Fmt corrected and passed. Native DLL built 2026-10-24 06:10:52, 20658688 bytes.
- Run record allocation: maximum local docs/runs run number was 433; recheck before assigning a record.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-gzmsg-shadow8 (queue manager shadow) | Codex / `glue-gzmsg-shadow8` | in progress | Inspect public sampling contracts; implement and measure available state. |

## Handoff
- Goal: default-off read-only gzmsg shadow of the queue manager, city load and monthly through 2032.
- Done: implemented default-off `gzmsg_shadow` load/month hooks and private Rust core lookup comparison; filed GZ11 for absent full-state observation contract.
- Scope: 259 explicit IDs (0..255, service ID, IID, UINT32_MAX), immutable numeric load baseline, monthly return/pointer equality. No native references retained; no resyncs; no whole-owner credit.
- Done: implementation committed as `488be884c`; focused tests 2/2 and explicit absolute-path fmt passed.
- Gate: `qbuild all` terminal PASS: Rust 6940 passed, 0 failed; clippy 0 diagnostics; coordinator native build 250/250 edges and CTest 78/78. Logs build-runs/qbuild-{test,clippy}.log and fc4verify-{build,ctest}.log. Native reported tree changed: only lane documentation changed during the gate, no source/input changes.
- Next: submit prepared `gzmsg-shadow8-r1-2032` request ending 2032-01-01; wait with build-runs/wait-game.ps1 (task forbids qwait for new requests).
- Facts: no game request outstanding. Prepared request build-runs/gzmsg-shadow8-r1-2032.pending. q.py sandbox Bash failure preserved; direct Cargo succeeded. Fmt corrected and passed.
- Run record allocation: maximum local docs/runs run number was 433; recheck before assigning a record.

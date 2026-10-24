| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-gzmsg-shadow8 (queue manager shadow) | Codex / `glue-gzmsg-shadow8` | in progress | Inspect public sampling contracts; implement and measure available state. |

## Handoff
- Goal: default-off read-only gzmsg shadow of the queue manager, city load and monthly through 2032.
- Done: implemented default-off `gzmsg_shadow` load/month hooks and private Rust core lookup comparison; filed GZ11 for absent full-state observation contract.
- Scope: 259 explicit IDs (0..255, service ID, IID, UINT32_MAX), immutable numeric load baseline, monthly return/pointer equality. No native references retained; no resyncs; no whole-owner credit.
- Done: implementation committed as `488be884c`; focused tests 2/2 and explicit absolute-path fmt passed.
- Gate: `qbuild all` terminal PASS: Rust 6940 passed, 0 failed; clippy 0 diagnostics; coordinator native build 250/250 edges and CTest 78/78. Logs build-runs/qbuild-{test,clippy}.log and fc4verify-{build,ctest}.log. Native reported tree changed: only lane documentation changed during the gate, no source/input changes.
- Game: `gzmsg-shadow8-r1-2032` terminal clean to 2032-01-08; foreign_exceptions=0, ref_warnings=0, resyncs=0, BUT gzmsg 0/73: native manager acquisition unavailable throughout. Run 434 records failed shadow evidence. GZ12 filed; no whole-owner or bounded lookup verification established.
- Diagnostic correction: register gzmsg_shadow in known config keys (first run logged three unknown-key warnings); add acquisition failure fields. Native alternative EnumSystemServices has explicitly unknown SDK callback signature; cannot safely proceed without GZ12.
- Final-source gate: qbuild all PASS on `9545111e0`: Rust 6940/0, clippy 0 diagnostics, coordinator build 206/206 edges, CTest 78/78. Same-operation native request `qbuild-20261024-061652-build-ctest` completed, no source changes during gate.
- Next: collect final-source `gzmsg-shadow8-r2-2032` game verdict and diagnostic fields. End WIP if no manager can be sampled; no ready queue credit for unavailable state.
- Facts: ONE outstanding game request `gzmsg-shadow8-r2-2032` (turbo Konradshohe to 2032). Wait via build-runs/wait-game.ps1, never submit a duplicate; an existing verdict is valid.
- Facts: q.py sandbox Bash failure preserved; direct Cargo succeeded. Fmt corrected and passed. Native DLL built 2026-10-24 06:10:52, 20658688 bytes.
- Run record allocation: 434 used for first run; recheck next free number before final record.

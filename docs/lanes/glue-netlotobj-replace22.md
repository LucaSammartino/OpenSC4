| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netlotobj-replace22: network lot ownership | Codex / `glue-netlotobj-replace22` (engine) | Ready measured partial: cargo 7641 passed; clippy 0 diagnostics; CTest 91/91; netmgr shadow 73/73; documented house32 clean through 2032, ref_warnings=0 | Complete crew/configuration bridge contracts; confirm current house profile |

## Report

- Lot views now retain their occupant and share ownership of their configuration, independently of the Rust manager's cell entry.
- `GetNetworkLot` returns a borrowed cached view without leaking a caller reference; changed occupant/configuration pairs replace stale views. Reference counts are atomic.
- Added DLL registration/ownership tests (seam operation 75): manager factories, repeated borrowed returns, cell replacement, retained lots after manager destruction, and configuration lifetime.
- Full `qbuild all` passed: Rust 7641 passed, 0 failed, 0 ignored; clippy 0 diagnostics; native build 159/159 edges; CTest 91/91. Explicit queued `qbuild verify` also passed 91/91.
- Exact shadow line: `netlotobj22-r1-shadow-20261025 73/73`; 0 configuration differences and 0 resyncs.
- Shadow verdict: clean at 2032-01-06, ref_warnings=0; record `build-runs/game-results/netlotobj22-r1-shadow-20261025.verdict` and matching `.log`.
- Replacement verdict: `netlotobj22-r1-house32-20261025`, clean at 2032-01-01, ref_warnings=0; record `build-runs/game-results/netlotobj22-r1-house32-20261025.verdict` and matching `.log`.
- House32 uses the 32-key set reconstructed from allowed operations notes and the subsystem registry. Its exact equality to the current forbidden regression script is unconfirmed; the request preserves every key used.
- Still missing: crew creation/interface contract and Rust-backed crew construction; the configuration COM bridge remains incomplete. No full three-class coverage flip is claimed.
- The network lot/configuration views were already plugin-created; this change fixes their ownership and borrowed-return behavior. The existing replacement keys and default-off shadow setting are unchanged.
- Questions filed: none, per the latest lane rules. Missing crew contract is marked with `// SPEC-GAP:` in code.
- C ABI exports unchanged. No coverage-map, regression-script, README, or run-record edits.
- Next: review/land this measured ownership fix; supply the crew contract, complete the configuration bridge, and confirm the house profile before crediting the entire requested flip.

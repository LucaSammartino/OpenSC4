| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-key-accelerator4: cRZWinKeyAccelerator core | `core-key-accelerator4` / detached reserved worktree (engine lane) | in progress | 1. Baseline gate. 2. Independent fixtures and owner inventory. 3. Implement and validate. |

## Handoff
- Goal: implement only widget-toolkit section 13 accelerator owner in a new Rust core module.
- Worktree reused at coordinator-reserved detached HEAD; clean at start, no commits beyond main.
- Read section 13, keyboard message section 7.3, all 16 ABI rows and public SDK headers.
- Independent fixtures written before implementation: 28 key/modifier cases, 9 asymmetric presence cases, duplicate/deletion/order/capacity, message, dispatch and lifetime traces. Not yet compiled.
- Filed WT7 (owner/lifetime inventory), WT8 (boundary/repeat/failure contract), WT9 (SDK/spec slot conflicts); index regeneration deferred to coordinator because tools/ is forbidden.
- Baseline qbuild all running in exec session 96460; preserve its logs.
- Next: implement production module from fixtures and literal contract; inventory full-owner residuals honestly.
- Scope: pure core, no C ABI changes, game runs, native-exact or replacement claims.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-key-accelerator4-r1-manifest.json.
- All owner gaps must remain explicit; no other widget/HTML owners.

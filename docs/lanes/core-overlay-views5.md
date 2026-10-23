| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-overlay-views5: overlay view owners | Codex, `core-overlay-views5` | in progress | Implement pure Rust owners and conformance tests |

## Handoff
- Goal: close overlay-views and network-overlay-views owners in engine/core/src/overlay_views; no ABI or plugin wiring.
- Worktree reused; detached HEAD is intentional. Initial tree clean; no lane commits above main.
- Next: read specified sections, core conventions and question numbering; implement and test.
- Required final checks: qbuild all and queued qbuild verify. Pure core changes need no game run.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-overlay-views5-r1-manifest.json.

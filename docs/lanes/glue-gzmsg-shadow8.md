| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-gzmsg-shadow8 (queue manager shadow) | Codex / `glue-gzmsg-shadow8` | in progress | Inspect public sampling contracts; implement and measure available state. |

## Handoff
- Goal: default-off read-only gzmsg shadow of the queue manager, city load and monthly through 2032.
- Done: implemented default-off `gzmsg_shadow` load/month hooks and private Rust core lookup comparison; filed GZ11 for absent full-state observation contract.
- Scope: 259 explicit IDs (0..255, service ID, IID, UINT32_MAX), immutable numeric load baseline, monthly return/pointer equality. No native references retained; no resyncs; no whole-owner credit.
- Next: collect focused cargo test session 83069; finish explicit-file fmt; commit and run stable-source all/verify, then unique turbo request to 2032.
- Facts: no game request outstanding. q.py failed because sandbox denied Bash CreateFileMapping; direct Cargo session active. Initial fmt with relative path failed; absolute fmt diagnosed formatting now patched.

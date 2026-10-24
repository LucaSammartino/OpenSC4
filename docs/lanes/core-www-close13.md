| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-www-close13: WWW owner closure | Codex / `core-www-close13` | in progress | Audit spec and implement missing owner contracts |

## Handoff
- Goal: close WWWSystem, HTTPClient, CookieManager and NetworkFileCache core owners without ABI/plugin changes.
- Done: reused reserved detached worktree; clean status; read lane rules and report format.
- Done: inspected spec sections 3–6, linked HTTP lifetime and WWW checklist, existing owner table and attribution evidence. Existing conformance tests cover CK01–06, WWW01–20, HTTP01–09 and NFC01–12.
- Next: complete settled HTTP directory/cache-setting state and WWW forwarding, audit lifecycle/vector edges, and make every unresolved row explicit in closure report.
- Facts: spec still explicitly parks WWW Q7/Q9; cookie Q1/Q3, HTTP Q13 and cache Q11 remain open. Do not invent these contracts or claim native whole-owner closure.
- Required final gates: qbuild fmt (absolute owned Rust paths), all and verify; no game wiring requested.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-www-close13-r1-manifest.json

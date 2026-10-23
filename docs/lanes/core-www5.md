| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-www5: web/network owner closure | Codex, core-www5 | in progress | Inspect existing core and spec; implement owner closure |

## Handoff
- Goal: close WWWSystem, HTTPClient, CookieManager and NetworkFileCache in core only.
- Existing reserved detached worktree reused; initial status clean.
- Next: inspect www/rzurl modules and assigned spec sections, then implement and test.
- No game requests; no ABI or plugin changes authorized.

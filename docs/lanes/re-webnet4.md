| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-webnet4: web and network framework spec (RE) | re-webnet4 / .claude/worktrees/re-webnet4 | in progress | write docs/specs/web-network.md |

## Handoff
- goal: spec the web/network framework family (86144 B uncovered) in docs/specs/web-network.md; no code.
- per-owner bytes (coverage.py --owners): cGZWWWSystem 34049, cRZHTTPClient 15133, cRZURL 9498, cRZCookieManager 8137, NetworkFileCache 6699, cRZNetSocket 4049, cRZNetCommandGenerator 3123, cRZNameLookup 2799, cRZNetStreamSocket 1700, HTTPMessage 957.
- done: web-network.md §1-2 (cRZURL) written
- next: §3 cRZCookieManager from decomp/cRZCookieManager.c + CookieData.c; REA had no binary open (export-first).

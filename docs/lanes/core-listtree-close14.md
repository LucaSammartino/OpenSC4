| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-listtree-close14 (ListBox and TreeView owner closure) | `Codex` / `core-listtree-close14` (engine lane) | in progress | (1) Audit spec slots and literal vectors; (2) implement and test gaps; (3) run full gates and enqueue. |

## Handoff

- Goal: close cGZWinListBox and cGZWinTreeView in core only, with complete owner/slot/vector coverage.
- Worktree: reserved detached HEAD; clean at start, no commits ahead of main.
- Done: audited widget-controls 5.1-5.6, 6.1-6.7 and 7 against owner methods and existing grouped tests.
- Findings: negative default-color indices all rejected; TreeView paint passes zero bar thickness and never updates buffer/bar requirements; Init silently tolerates stale root after Shutdown.
- Next: implement raw property aliases, integrated layout and explicit stale-root precondition; add missing literal vectors and slot adapter tests.
- Scope: do not edit plugin, capi, coverage-map, FileBrowser or TextEdit rows; ABI unchanged.
- Gates: final qbuild all plus coordinator-queued qbuild verify; no game request required for core-only work.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-listtree-close14-r1-manifest.json.

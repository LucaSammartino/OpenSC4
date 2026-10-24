| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-listtree-close14 (ListBox and TreeView owner closure) | `Codex` / `core-listtree-close14` (engine lane) | in progress | (1) Audit spec slots and literal vectors; (2) implement and test gaps; (3) run full gates and enqueue. |

## Handoff

- Goal: close cGZWinListBox and cGZWinTreeView in core only, with complete owner/slot/vector coverage.
- Worktree: reserved detached HEAD; clean at start, no commits ahead of main.
- Done: audit checkpoint f57839fa5; implemented raw property aliases/stored-pair/base-color adapters, checked stale-root Init, provisional automatic buffer/bar layout and additional literal vector/helper tests.
- Done: module README now has ListBox slot map and updated owner closure rows; WC7 filed for exact buffer/bar recalculation policy.
- Stop: two-thirds context threshold reached; no new implementation work. TreeView primary-wrapper and exhaustive assertion audit remain partial.
- Done: implementation commit e8020b3e3; six-file final formatting PASS. Focused session 9875 collected to terminal: exit 101, obsolete helper-name error; current source uses move_to.
- Pending: stable-source qbuild all started 2026-10-24 22:50:18 with CARGO_BUILD_JOBS=1; session 90505 remains active. At 23:46:35 only compilation progress (core/formats/conformance/capi), no test summary. Log: build-runs/qbuild-test.log. No duplicate full gate launched.
- Next: collect session 90505 to terminal, read PASS/FAIL lines and bounded failure excerpts; then explicitly run queued qbuild verify. Resolve diagnostics only; finalize Report + Handoff and merge queue (ready only if gates pass).
- Evidence: q Bash startup failed in build-runs/q/20261024-222624-28284.log. Original focused failure retained in build-runs/focused-widget-controls.log.
- Scope: do not edit plugin, capi, coverage-map, FileBrowser or TextEdit rows; ABI unchanged.
- Gates: final qbuild all plus coordinator-queued qbuild verify; no game request required for core-only work.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-listtree-close14-r1-manifest.json.

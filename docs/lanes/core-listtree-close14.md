| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-listtree-close14 (ListBox and TreeView owner closure) | `Codex` / `core-listtree-close14` (engine lane) | in progress | (1) Audit spec slots and literal vectors; (2) implement and test gaps; (3) run full gates and enqueue. |

## Handoff

- Goal: close cGZWinListBox and cGZWinTreeView in core only, with complete owner/slot/vector coverage.
- Worktree: reserved detached HEAD; clean at start, no commits ahead of main.
- Done: audit checkpoint f57839fa5; implemented raw property aliases/stored-pair/base-color adapters, checked stale-root Init, provisional automatic buffer/bar layout and additional literal vector/helper tests.
- Done: module README now has ListBox slot map and updated owner closure rows; WC7 filed for exact buffer/bar recalculation policy.
- Stop: two-thirds context threshold reached; no new implementation work. TreeView primary-wrapper and exhaustive assertion audit remain partial.
- Next: collect focused cargo session 9875 to terminal; then qbuild all on corrected stable source and queued qbuild verify; resolve diagnostics only, finalize report/queue.
- Evidence: q Bash startup failed in build-runs/q/20261024-222624-28284.log. Focused build log build-runs/focused-widget-controls.log has old-source helper-name error; fixed to move_to while its other compile job finishes. Formatting passes for six owned Rust files (rerun after final offset assertions).
- Scope: do not edit plugin, capi, coverage-map, FileBrowser or TextEdit rows; ABI unchanged.
- Gates: final qbuild all plus coordinator-queued qbuild verify; no game request required for core-only work.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-listtree-close14-r1-manifest.json.

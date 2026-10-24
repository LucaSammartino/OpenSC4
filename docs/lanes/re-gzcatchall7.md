| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzcatchall7: coverage-map row order, the `cGZ*` catch-all moved below the rows it hid (RE coverage lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-streamops7) | **ready** (map only). `cGZDate` (dates.md §3) and the four command-system owners now reach their rows: specified +4,316 bytes (+100 functions) | none |

## Report

The rows `cGZDate` → dates.md and `cGZCommandParameterSet`, `cGZStringCommandGenerator`,
`cGZMessage2CommandDispatcher`, `cGZMessage2CommandDispatcherMessage` → command-system.md sat below the `cGZ*`
catch-all, so the first-match rule never reached them (true on main `4aa46040` too). A scan of every literal row
against the rows above it found no other hidden row (the `m:` method rows are matched before owner rows by
tools/coverage.py and are not affected). Both specs have these classes as their subject (dates.md title and §3,
command-system.md title and §2–§4). Written on 2026-10-24.

### What changed

- tools/coverage-map.tsv: `cGZ*` moved from above `cRZFile` to below `fn:RZParseDelimitedText`, with a comment.

### Coverage

- Specified bytes 4,852,744 → 4,857,060 (+4,316, 87.0%, `cov_local.py`).

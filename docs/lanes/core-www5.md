| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-www5: web/network owner closure | Codex, core-www5 | in progress | Inspect existing core and spec; implement owner closure |

## Handoff
- Goal: close WWWSystem, HTTPClient, CookieManager and NetworkFileCache in core only.
- Done: a82e0ea7 initial handoff; implemented cookie/cache owners and HTTP notification/calendar helpers.
- Extended existing WWW FTP admission and HTTP worker exit; added owner closure table and vector index.
- Focused tests: 41 then 42 WWW tests passed; later sized-buffer/info tests and lifecycle edits await final gate.
- Q15 appended to docs/specs/questions/web-network.md: literal splitter/settings/writer edge choices.
- Existing spec limits remain Q1/Q3/Q7/Q9/Q10/Q11/Q13, documented by section in www/README.md.
- Formatting applied via apply_patch from rustfmt-generated diffs (never wrote formatter output into source).
- q.py failed before cargo because Git Bash CreateFileMapping denied; direct cargo worked.
- qbuild fmt rejected relative file inputs (runs from engine); next use absolute owned paths.
- Next: absolute-path qbuild fmt, then qbuild all ONCE on stable source; inspect actual terminal result.
- No game requests; unconnected core only, no ABI/plugin changes; detached HEAD retained.

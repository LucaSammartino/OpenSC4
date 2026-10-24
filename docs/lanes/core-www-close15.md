| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-www-close15 (WWW owner closure) | Codex / `core-www-close15` (engine lane) | in progress | Resolve queued spec merges; implement settled contracts; run gates. |

## Handoff
Goal: merge re-www-q17 then re-www-q18 and close the five WWW owners in core only.
Done: merged q17 at b8ded3ec5 and q18 at 8a66a839c; resolved overlapping indexes.
Done: cookie splitter, hex persistence, constructor default and changed-path reload implemented; CK07-CK13 fixtures added.
Done: HTTP NORM01-08, WWW null-target Q10 and Cookie-header construction implemented.
Done: HTTP fixed-position date parser, builder, framing selector, retry/finalization helpers and constructed fixtures added.
Tests: initial cookie fixture 6/7; wrong test expectation for derived / fixed. Evidence build-runs/cookie-initial-result.txt.
Next: collect focus-www.py result; implement HTTP stream/framing worker, slots/destructor; FTP commands/support; cache Q11/Q15.
Formatting: format-patch.py applies bounded rustfmt diffs via apply_patch; use absolute paths for qbuild fmt (relative paths resolve under engine).
Facts: no ABI/plugin/capi changes; no game requests; manifest build-runs/run4-core-www-close15-r1-manifest.json in main.

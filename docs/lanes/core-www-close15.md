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
Done: HTTP body framing/spill, QueryInterface/deleting wrapper/destructor helpers and WWW-21/22 fixtures implemented.
Tests: focus-www-3.log PASS 33/33 (7 cookie, 2 HTTP notification, 5 lifetime, 5 settled, 14 WWW).
Failed evidence: focus-www-1.log obsolete Unsupported PUT assertion; focus-www-2.log fixture method-name typo; both corrected.
Next: implement HTTP wire worker/header reader integrating http_wire helpers; FTP commands/support; cache Q11/Q15. Update README closure rows.
Formatting: format-patch.py applies bounded rustfmt diffs via apply_patch; use absolute paths for qbuild fmt (relative paths resolve under engine).
Facts: no ABI/plugin/capi changes; no game requests; manifest build-runs/run4-core-www-close15-r1-manifest.json in main.

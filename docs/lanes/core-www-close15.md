| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-www-close15 (WWW owner closure) | Codex / `core-www-close15` (engine lane) | in progress; focused 33/33 and FTP FTPC01-06 pass; fmt 14 files clean; no ABI changes | Continue HTTP worker, FTP support and cache closure, then all/verify gates. |

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
Done: a1188de1a HTTP framing/slots/destructor milestone; 33 focused tests passed before the CopyHeaders observer addition.
Done: FTP seven command bodies/dispatcher and scheduler construction/admission added, FTPC01-08 tests; module README distinguishes settled-but-pending engine work.
Tests: 20918 terminal PASS FTPC01-06; focus-www-4.log terminal PASS 33/33 including CopyHeaders; qbuild fmt PASS 14 files, zero diffs.
Next: collect existing cargo session 78055 to terminal (filter www::ftp::tests::ftpc; FTPC01-08); do not duplicate it. Full all/verify gates not yet run: source is not final.
Next: implement HTTP transport worker/header reader; FTP remaining support (4.15), fix stale initialize/close/name barriers; cache Q11/Q15 and NFC13-19.
Facts: Session.initialize incorrectly labels scheduler-constructor address 00813d70; compare FTP-control §6 vs §9.1 before resolving this remaining integration.
Formatting: format-patch.py applies bounded rustfmt diffs via apply_patch; use absolute paths for qbuild fmt (relative paths resolve under engine).
Facts: no ABI/plugin/capi changes; no game requests; manifest build-runs/run4-core-www-close15-r1-manifest.json in main.
Closure: CookieManager and CookieData published portable behavior covered; WWW/HTTP/cache not fully implemented. No READY entry exists.

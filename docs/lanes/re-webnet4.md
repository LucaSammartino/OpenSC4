| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-webnet4: web and network spec (RE), web-network.md: cRZURL full, cRZCookieManager parse/add/lookup | re-webnet4 / .claude/worktrees/re-webnet4 | ready (docs) | answer web-network Q1-Q3; spec cRZHTTPClient, cGZWWWSystem + FTP client |

## Report

What changed
- New docs/specs/web-network.md. Section 2 covers cRZURL end to end: state, type codes, the scheme step with guess mode
  and allowRelative mode, the user-info, server/port and path steps, worked examples, rebuilding the URL, resolving
  a relative URL against a base, and path normalisation. Section 3 covers cRZCookieManager: the record defaults, Set-Cookie
  parsing, AddCookie validation (RFC 2109), lookup matching, the Cookie header text and eviction. Section 4 outlines
  cGZWWWSystem (including the FTP client it reaches, about 30 KB) and cRZHTTPClient.
- Questions: docs/specs/questions/web-network.md Q1 to Q3 (Port list length, store capacity, cookie file format); QUESTIONS.md reindexed.
- Map proposal (not applied, no tools/ edit): build-runs/map-rows-re-webnet4.tsv sets spec web-network.md on the
  `cRZURL` row only (9498 B). cRZCookieManager stays unmapped until §3.6 (file format, capacity) is written.

Measured (coverage.py --owners, web and network family 86144 B): cGZWWWSystem 34049, cRZHTTPClient 15133, cRZURL
9498, cRZCookieManager 8137, NetworkFileCache 6699, cRZNetSocket 4049, cRZNetCommandGenerator 3123,
cRZNameLookup 2799, cRZNetStreamSocket 1700, HTTPMessage 957. Fully specified: cRZURL (9498). Partly: cRZCookieManager.

Evidence limitations: the REA preflight was GO, but batch_decompile returned target_unavailable (no binary was open
in the shared session). All evidence came from the Ghidra export first (re_query asm/xrefs/callees), decomp/ class
files and decomp/_unnamed region files. Key branches were checked in the asm (mailto type at 00822246, user-info copy at
0082273a, the string constructor setting allowRelative at 00821c6d, cookie attribute targets at 008f968a..008f9709,
version move at 008f76c5). No live network request was made.

Quirks recorded (they must be reproduced for parity): https parses as type 2; `mailto:` leaves the type at 0; user-info
keeps the leading `//` and leaves the server empty; Discard cookies never match; secure cookies never match; the
version attribute is moved to the front of the header again for every cookie.

Next
- Answer Q1 (read HTTPMessage::GetSeparatedStringList), Q2 (cRZCookieManager Init), Q3 (Read/WriteCookieDataFile).
  Then map cRZCookieManager and HTTPMessage.
- Spec cRZHTTPClient (request and response state machine, TimeStringToSeconds), then the cGZWWWSystem job API and
  the FTP client.

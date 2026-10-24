| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-www-close15 (WWW owner closure) | Codex / `core-www-close15` (engine lane) | ready for partial delivery; all PASS cargo 7228/0, clippy clean, CTest 81/81; explicit verify PASS build 165/165, CTest 81/81; Q19/Q20 filed; ABI unchanged; game n/a | (1) Resolve Q19/Q20. (2) Implement native WWW 4.15 container/deleting-array/interface wrappers and connect HTTP worker resource publication. |

## Changes

Resumed the existing lane and ran the required merges in order. Both re-www-q17 and re-www-q18 were already ancestors of this detached checkout (earlier merge commits b8ded3ec5 and 8a66a839c); merge commands reported already up to date. Shared packed-refs.lock warnings reflect the read-only shared refs; detached commits succeeded without changing branches or refs.

The earlier round implemented settled cookies, CookieData copies, request normalization, WWW null-target behavior, cookie construction, HTTP wire/framing/finalization/calendar/lifetime helpers, FTP command bodies and scheduler admission. This round added:

- Cache Q11/Q15: FileCache.ini default; atomic CREATE_NEW filename candidates with signed time/decrementing collisions; source-file admission and destructive replacement; rightmost-seven-comma INI loading; expiration/length acceptance and file cleanup; case-sensitive orphan membership; direct writer materialization and host-supplied native bucket/chain traversal. NFC13–NFC19 now execute.
- Plain-stream HTTP header accumulation and transaction orchestration: short and zero sends, numeric/name resolution, low-16-bit port/case-insensitive server reuse, status-zero rejection, optional informational skipping (worker disables it), cancellation/deadline handling, framing/body readers, retry and redirect admission.
- Settled FTP identity/listener/release/control-socket/name-wrapper helpers, listing close-helper continuation, scheduler destruction and range erasure, wrapping worker reference adjustment.
- Name lifecycle/ascending removal, little-endian address/port views, bounded formatter results, synchronous borrowed progress and its 1/50 quirk, and drain/log/counter/close ordering. SUP01–SUP09 have constructed fixtures.

Implementation milestones: ec25b05e4 (cache), b21e621cf (HTTP/support), 9e16b0f8a (format/closure matrix), c84351463 (clippy fixture correction), 0ef4db265 and 8c74a39b4 (gate evidence). The module README maps each published owner section, including conformance sections, to code/tests or its explicit remaining boundary.

## Owner closure

| Owner | Fully implemented? | Exact disposition |
|---|---|---|
| cGZWWWSystem | No | Portable front-end 4.1–4.14 behavior and qualified 4.15 support covered. Fresh FTP session initialization and conflicting listing-helper labels are Q19. Native 4.15 circular-sentinel/list allocation, deleting-array and pointer-adjusting adapter wrappers remain implementation work; these are settled contracts, not unanswered questions. |
| HTTP / cRZHTTPClient | No | Admission, normalization, notifications, lifetime, dates, body framing, finalization and transport algorithms are implemented/tested. Native dequeue/response-resource publication and record-field transition, proxy endpoint selection and malformed status-line grammar remain Q20; no invented request-to-resource cast connects those boundaries. |
| cRZCookieManager | Yes, published portable behavior | Sections 3.1–3.7 and CK01–CK13 covered; native IO/allocation/calendar primitives remain host inputs. |
| CookieData | Yes, published value behavior | Section 3.1 defaults, strings/fields and independent value copying covered. |
| NetworkFileCache | Yes, published portable storage behavior | Sections 6.1–6.5 and NFC01–NFC19 covered. Filesystem primitives and native bucket topology are explicit host inputs; Arc buffer ownership still requires the native allocator/16-bit-reference adapter. |

This is a passing partial delivery, not full five-owner or native-equivalence closure. New implementation stopped around two thirds of the measured context budget as instructed; the remaining time was used for diagnostics, gates and reporting.

## Validation

- Focused r2: cache 6/6; transport/cache/WWW 24/24; FTP 15/15 (including SUP identity/copy/range/wrapping/destruction assertions).
- Formatting: 9 explicit owned Rust files, zero diffs; corrected transport fixture rechecked separately, zero diffs.
- Initial full gate: cargo 7228 passed / 0 failed / 0 ignored; four clippy field_reassign_with_default diagnostics in new fixtures (corrected to struct initialization); native build 262/262 and CTest 81/81 passed. Overall FAIL was preserved, not replaced.
- Corrected stable-source all: cargo 7228 passed / 0 failed / 0 ignored (sum of every test-result line), clippy zero diagnostics, native build 46/46 and CTest 81/81; qbuild PASS.
- Required explicit verify: qbuild-20261024-151616-build-ctest, verdict pass; build 165/165 (9.1 s), CTest 81/81 (95.6 s), total 107.1 s; qbuild PASS.
- git diff --check clean. No operation remains outstanding.

Evidence is under this worktree's build-runs: all-r2.log and *-r2-initial.log preserve the failed first gate and native pass; all-r2-fixed.log, qbuild-test.log, qbuild-clippy.log and *-r2-all-final.log preserve corrected all; verify-r2.log and fc4verify-{build,ctest}.log preserve explicit verify. Focused logs: cache-r2.log, transport-r2-fixed.log, ftp-r2-final.log. Earlier fixture failure transport-r2.log and q.py bash CreateFileMapping failures remain preserved.

## Questions and limits

Q19 and Q20 were appended to docs/specs/questions/web-network.md with provenance. Q19 keeps fresh-session initialization Unsupported; listing follows the newer parent close-helper contract literally. Q20 records the literal HTTP status/version parsing choice and the missing native record/resource and proxy endpoint qualification. Q12 (command generator) remains outside these five owners. The generated question index was not edited or regenerated with prohibited tools.

No ABI exports, version, capi or plugin wiring changed. No real-game request was submitted: these portable modules have no shadow or replacement key and no integration in the plugin. Game verdict, reference warnings and shadow equivalence are n/a; constructed fixtures are not live-game observations.

## Next steps

Resolve Q19/Q20 from spec-side evidence, complete the remaining settled WWW 4.15 native wrappers, then connect the HTTP dequeue/response-resource path and requalify the two partial owners. A READY entry intentionally lands only the tested partial implementation; it must not be counted as full WWW or HTTP owner coverage.

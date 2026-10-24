| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-helicopter-shadow10: helicopter shadow | Codex, glue-helicopter-shadow10 | in progress | Inspect contracts and implement supported shadow comparisons. |

## Handoff
- Goal: default-off helicopter shadow, measured through 2032 with house switches.
- Done: independent class/four-version projection, default-off ini key, saved-manager scan, private COM metadata fallback; AF10 filed.
- Focused C ABI test: 1 passed (null/short/failing stream and exact nested versions); explicit-file fmt passed; diff check clean.
- Implementation commit: d3f449106.
- Stable-source qbuild all: cargo 7012 passed/0 failed/0 ignored; clippy 0 diagnostics.
- Native queue qbuild-20261024-101914-build-ctest terminal PASS: build 255/255 edges (140.5 s), CTest 80/80 (223.1 s); qbuild PASS. Same all operation performs the coordinated verify; no duplicate native request.
- Gate logs: build-runs/qbuild-{test,clippy}.log and fc4verify-{build,ctest}.log. Checker tree-change note was only lane-doc commit f057e228a; implementation source unchanged.
- First game helicopter10-r1-20261024: stalled at 2027-02-20, helicopter 13/13, 39 reference warnings, foreign exceptions 0; run record docs/runs/helicopter10-r1-20261024-10479.md. Evidence retained.
- Diagnostic fixes: audit acquired COM factory reference before created release; use simulator date at PostCityInit when PublishedJulian is zero. Source changed after the first gate and run.
- Corrected implementation f23e231fb: full qbuild all PASS, 7012 Rust tests/0 failed/0 ignored; clippy 0 diagnostics; queued native qbuild-20261024-103348-build-ctest PASS 47/47 edges, CTest 80/80 (88.1 s); no tree-change warning.
- Outstanding game request helicopter10-r2-20261024, submitted after corrected full gate; city Konradshohe, house ini plus helicopter_shadow=1, turbo to 2032, 2400 s limit. Do NOT resubmit or discard an existing verdict.
- Next: collect existing blocking wait (exec session 79551), or run build-runs/wait-helicopter.ps1 against game-results/helicopter10-r2-20261024.verdict; on its 9-minute timeout wait again without resubmitting. Then run build-runs/measure-helicopter.ps1 -Label helicopter10-r2-20261024, record free run number, finalize READY partial report/queue if clean.
- Blocking waiter uses FileSystemWatcher, not qwait or polling. First wait on r1 failed on absent results directory; fixed before actual wait. Analysis helper is read-only and prints bounded output.
- Scope: no parameter/home/goal/physics/lifecycle comparison or complete live enumeration (AF2-AF6/AF9/AF10).
- Fallback is NOT a live spawn; no specified safe scenario group/exemplar creation sequence (AF10).
- Initial fmt invocation used engine-relative paths from engine cwd and failed; absolute paths passed. q.py Bash launch failed Win32 access 5; direct cargo test passed.
- Restrictions: clean room; no tools/ reads or execution; no agents; native build through qbuild verify.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-helicopter-shadow10-r1-manifest.json.

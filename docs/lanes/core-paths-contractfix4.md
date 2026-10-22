| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-paths-contractfix4: PA4-PA8 portable path corrections | `Codex` / `core-paths-contractfix4` (engine lane) | in progress | Audit published contracts, correct portable paths, run full gate |

## Handoff
- Goal: apply PA4-PA8 to production paths and directory-entry lifecycle; reuse cRZDirectory.
- Scope: portable Rust only; no ABI/shared files, native glue, game runs, or whole-owner completion claim.
- Done 3032f628: first substantive PA4 correction preserves full counted ResolvePath input on the full-prefix branch; independent NUL/copy vector added (before call 30).
- Build evidence: baseline and earlier quick/all overlapped; use final-gate-result.json as final authority, not their mixed native logs.
- Done 1fc145c6: signed-byte CRT primitive boundary/errors; counted ResolvePath, entry default/copy/name-only drop, signed SplitDirectory/partial writes, retained find cache/retry/finish.
- Done: removed duplicate paths directory owner/comparator; paths reexports authoritative rz_files owner. Primitive-only test host bridges it. Added PA4-PA8 independent vectors.
- Tests: stable fmt recheck passed for all 7 Rust files after 8c6589ec; old quick compiled in 23m39s and is running tests. Sequential driver's quick/all results pending.
- Next: final-gate.py runs fmt/quick/all sequentially on stable source; inspect build-runs/final-gate-result.json and native logs, report exact scope and enqueue only if green.
- New questions PA9/PA10: negative signed capacity follows literal signed-loop no-output rule; zero-part SplitDirectoryPath fills both safe empty outputs. PA1/PA2/PA3 retained.
- Done 56d8bee9: nullable next boundary, CRT error propagation and signed split counts in conformance runner; literal JSON zero/one-byte/root vectors.
- Baseline caveat: long cold build saw updated runtime JSON with its old binary (zero-capacity vector failure); do not claim a clean baseline. Earlier overlapping all logs are not authoritative final evidence.
- Done 7476c5d9: remaining old high-bit/ASCII expectation fixed. Sequential driver owns final evidence; it is still in quick before final all.
- Done 8c6589ec: removed nullable-finish convenience (no published null-finish contract). Valid finish/Drop close once; null next is tested. Final all follows this correction and final EOF formatting.
- Audit complete: all published path constructors/destructor/getters/setters/joins and free helpers checked. Entry list remains authoritative passive payload; standalone lifecycle adds no native getters/COM/lifetime extension; RZ9-RZ11 and native layout/glue remain withheld.
- PA1/PA2/PA3 remain open; preserve selected normalization rule.
- Correction reservation logical round 2/3; this task permits one round only.
- q.py cannot launch Bash (E_ACCESSDENIED); use bounded PowerShell output and required qbuild wrappers.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-paths-contractfix4-r1-manifest.json.

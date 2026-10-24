| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-viewutil-shadow11: view utility shadow | Codex, glue-viewutil-shadow11 | in progress | Inspect contracts and implement supported comparisons. |

## Handoff
- Goal: default-off viewutil_shadow, monthly/load bit-exact comparison through 2032.
- Done: 16bdc1dc8 adds default-off rotation-only shadow and private production-core word adapter; filed VU16 (native callable ABI missing for 49/50 bodies, including min/max).
- Next: collect default-off control (seconds=1500) to isolate first run shutdown stall; then retry enabled under unique label with same allowance. Native gate PASS; all 70517 terminal exit 0. No source change or gate repeat needed for runtime-input diagnostic.
- Scope: only §4.3 explicitly documents cdecl. Compares 156 local-vector calls / 469 fields per available live sample (7 fixed + 5 live vectors, 12 fixed rotations + live rotation). No native state writes, no resync, no whole-owner credit.
- Evidence: q wrapper Bash CreateFileMapping denied (build-runs/q/20261024-124612-15784.log); direct PowerShell focused cargo operation 1243 canceled with terminal exit 1 after no output (no test verdict claimed). Absolute-path qbuild fmt passed.
- Rust gate: qbuild all reported test exit=0, 7141 passed / 0 failed / 0 ignored (45 result lines); clippy exit=0, diagnostics=0. Logs qbuild-test.log and qbuild-clippy.log. Native build/CTest queued at 13:10 local, all session 70517 still running.
- Game: no request outstanding. First enabled run returned stalled, last heartbeat 2030-11-15, ref_warnings=0 / exceptions=0. Exact line: viewutil11-r1-on-20261024-1255 59/59. All rows samples=156 fields=469 resyncs=0 mismatches=0; live available on load. Record docs/runs/viewutil11-r1-on-20261024-1255-508.md preserves failure. Wait sessions 11229 (timeout) and 23878 (verdict) both terminal. Use wait-verdict.ps1 for new unique labels, no qwait. Do not discard existing completed verdicts.
- Control OUTSTANDING: viewutil11-r1-off-20261024-1336, submitted with seconds=1500, end 2032, no viewutil key. Original first on-run 600s deadline quit before target and then quit failed to finish within 90s. All 53 house-shadow summary keys exact and refs/exceptions zero. Control isolates whether shutdown stall also occurs default-off. Use wait-verdict.ps1 -Label viewutil11-r1-off-20261024-1336; do not submit on retry until control verdict.
- Native gate: qbuild-20261024-131027-build-ctest PASS; build 262/262 edges (880.6s), CTest 81/81 (1270.7s). Logs fc4verify-build.log and fc4verify-ctest.log. qbuild all PASS terminal exit 0. Tree-change note is documentation-only: no engine diff from launch commit 64589b0bb to HEAD.
- Constraints: no tools/ access or edits; regress profile update belongs to coordinator because clean-room restriction forbids it. Native verify uses queued qbuild.

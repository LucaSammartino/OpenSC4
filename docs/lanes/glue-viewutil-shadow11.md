| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-viewutil-shadow11: view utility shadow | Codex, glue-viewutil-shadow11 | in progress | Inspect contracts and implement supported comparisons. |

## Handoff
- Goal: default-off viewutil_shadow, monthly/load bit-exact comparison through 2032.
- Done: 16bdc1dc8 adds default-off rotation-only shadow and private production-core word adapter; filed VU16 (native callable ABI missing for 49/50 bodies, including min/max).
- Next: collect stable-source qbuild all session 70517, now waiting on native queue qbuild-20261024-131027-build-ctest. DO NOT start a duplicate verify; all already queued build+ctest through the required coordinator path. Then run prepared house profile plus viewutil_shadow to 2032.
- Scope: only §4.3 explicitly documents cdecl. Compares 156 local-vector calls / 469 fields per available live sample (7 fixed + 5 live vectors, 12 fixed rotations + live rotation). No native state writes, no resync, no whole-owner credit.
- Evidence: q wrapper Bash CreateFileMapping denied (build-runs/q/20261024-124612-15784.log); direct PowerShell focused cargo operation 1243 canceled with terminal exit 1 after no output (no test verdict claimed). Absolute-path qbuild fmt passed.
- Rust gate: qbuild all reported test exit=0, 7141 passed / 0 failed / 0 ignored (45 result lines); clippy exit=0, diagnostics=0. Logs qbuild-test.log and qbuild-clippy.log. Native build/CTest queued at 13:10 local, all session 70517 still running.
- Game: no request outstanding. Prepared build-runs/viewutil11-r1-on-20261024-1255.pending (house ini switches, turbo, end 2032). Move to build-runs/game-queue/<label>.req with apply_patch only AFTER native PASS. Wait in one call using build-runs/wait-verdict.ps1 (FileSystemWatcher, no qwait per task final instruction). Keep any existing completed unique-label verdict.
- Control prepared: build-runs/viewutil11-r1-off-20261024-1336.pending. Run AFTER enabled verdict, one request at a time. INI inputs checked identical except on-only viewutil_shadow=1; off omits the key. Compare house shadow clean/total counts and verify off emits no viewutil lines.
- Native queue still has no verdict after ~30 minutes (13:40 local). Own request is valid: build-runs/build-queue/qbuild-20261024-131027-build-ctest.req, steps=build ctest. Keep collecting session 70517; do not submit duplicate verify. No native error is known.
- Constraints: no tools/ access or edits; regress profile update belongs to coordinator because clean-room restriction forbids it. Native verify uses queued qbuild.

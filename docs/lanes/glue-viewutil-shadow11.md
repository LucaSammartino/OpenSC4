| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-viewutil-shadow11: view utility shadow | Codex, glue-viewutil-shadow11 | in progress | Inspect contracts and implement supported comparisons. |

## Handoff
- Goal: default-off viewutil_shadow, monthly/load bit-exact comparison through 2032.
- Done: reserved detached worktree reused; added default-off rotation-only shadow and private production-core word adapter; filed VU16 (native callable ABI missing for 49/50 bodies, including min/max).
- Next: collect focused cargo operation session 1243 (functions cell 22), then stable-source qbuild all and queued verify; run house profile plus viewutil_shadow to 2032.
- Scope: only §4.3 explicitly documents cdecl. Compares 156 local-vector calls / 469 fields per available live sample (7 fixed + 5 live vectors, 12 fixed rotations + live rotation). No native state writes, no resync, no whole-owner credit.
- Evidence: q wrapper Bash CreateFileMapping denied (build-runs/q/20261024-124612-15784.log); direct PowerShell cargo test running. Absolute-path qbuild fmt passed.
- Constraints: no tools/ access or edits; regress profile update belongs to coordinator because clean-room restriction forbids it. Native verify uses queued qbuild.

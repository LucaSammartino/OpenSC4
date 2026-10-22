| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-paths-directories4: portable path and directory entry contracts | Codex / `core-paths-directories4` (engine lane) | ready; cargo 4307 passed; clippy 0 diagnostics; ctest 67/67; game n/a; PA4-PA8 filed; entry complete-owner credit withheld | 1. Review and merge portable code. 2. Resolve PA4-PA8 before native binding or entry complete-owner credit. |

## Changes and scope

Added `engine/core/src/paths/` from public paths.md sections 1-4 and 8. Parsing uses framework bytes, ASCII letters, exact comparisons and both separator spellings. It preserves the published drive-root, dot-component, reverse-split and last-joined-buffer quirks. C-output helpers count the NUL, preserve caller storage on failure, permit omitted split outputs, and return independent allocations. No host filesystem parser, locale folding or wildcard emulation supplies owner results.

The initial audit found no existing path owner/helpers or rz_services module in this checkout. Reused `rz_files::OsHandle` and `OsError`; the existing file host has no directory enumeration primitive. New `DirectoryProvider` exposes only native first/next/finish operations. Its native adapter must perform the specified encoding conversion and system wildcard matching/order. The core owns search state, filtering, dot synthesis and handle closure, including early exits. No CmdLine/SysServPtr work, C ABI/plugin changes, runtime binding, maps or coverage edits.

## Individual owner checklist

- **cRZPath:** all published constructor/lifecycle and method contracts implemented: five owned strings initialized before SetPath; Rust destruction of all five strings; SetPath (always true, preserve old buffer on resolution failure, then resplit); GetDrive/GetDirectory/GetFileName/GetExtension; lazy FileName; SetFileName/SetExtension split/equality behavior; SetBuffer masks; GetPath/GetDriveAndDirectory/GetFileNameAndExtension with independent output copies. All three in-place part-fixing helpers are shared with the free helper API. PA4 explicitly records byte/encoding/NUL/locale assumptions where the spec says only “characters” or cRZString.
- **cRZDirectoryEntry:** published owner/name/kind state implemented; names belong to the directory, kind is 4/8, and entry views borrow the actual owner. Safe Rust prevents retaining a dangling entry across reread, mutation or destruction. **Do not award complete-owner credit:** §3.1 publishes no standalone entry methods, copy/assignment or native lifetime contract. PA5 requests those facts; none were invented.
- **cRZDirectory:** only the entry/list support in §§3.1-3.4 is implemented to support the entry contract. Recursive traversal, existence/create/remove operations are outside this owner task and receive no implementation/complete-owner claim.

## Validation

- Stable `qbuild quick` at 1563e7a1: **3630 passed, 0 failed**, dependents compiled, native build 4/4 edges.
- One stable `qbuild all` at 9c0b8673: **4307 passed, 0 failed, 0 ignored; clippy 0 diagnostics; native build 2/2; CTest 67/67 (84.5 seconds)**. No tracked files changed during either passing run. Final commit changes this report only.
- Added **15 core tests** for empty/boundary/drive/UNC/dot/separator/byte/size/ownership/list/search cases and **3 language-neutral conformance vectors with 38 operations**, all passing. The first quick found one incorrect C: expectation, corrected to the literal §1.2 trailing-separator rule before the stable runs.
- `qbuild fmt` passes for all six new Rust modules. Checking the three shared registration files reports 172 ordering/layout diff lines, including existing module ordering and the required appended registrations. Preserved additive edits; no mass reformat. `git diff --check` clean.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-ctest.log`, `fc4verify-build.log`, and `fc4verify-quick-{test,deps}.log`.
- Game runs: **n/a**, core-only lane (`game: false` in manifest); no runtime shadow/replacement claim. C ABI exports unchanged.

## Questions and next steps

Appended **PA4-PA8** to `docs/specs/questions/paths.md`: byte/string/locale boundaries; missing entry owner contracts; SplitDirectory zero/count/component boundaries; failed-next cached state; comparator arguments/ties. Choices are marked in code. Existing PA3 remains open; explicit normalization §§1.2/6.9 is followed without reconciling its conflicting §8 example.

Review/merge the portable implementation, resolve these questions before native binding or entry complete-owner credit, then add the native encoding/find adapter only under a separately authorized glue task. No pending game requests.

Implementation commits: 226c963c (owners/helpers/entry state), 1563e7a1 (buffers/fix helpers/conformance), 9c0b8673 (focused gate record). Reserved detached HEAD was reused; supervisor publishes it. `q.py` cannot start Bash here, so capped native PowerShell reads and authorized qbuild were used. Git commits succeed despite a packed-refs.lock permission diagnostic; no shared refs/config were changed.

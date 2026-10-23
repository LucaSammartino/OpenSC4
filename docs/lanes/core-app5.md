| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-app5: cSC4App owner closure | Codex / core-app5 (detached HEAD) | blocked: native gate stalls at CMake compiler ABI detection; Rust tests/clippy/fmt pass; spec gaps explicitly documented | Coordinator rerun native gate in an environment where CMake/Ninja can complete compiler probes; resolve AP12-AP15 and existing application questions before claiming complete native owner coverage. |

## Report

Extended the existing portable application owner; no capi/plugin wiring, ABI exports,
ABI version, coverage map or generated ABI-entry changes. The GZ framework, COM
directors, city/simulator/region/window objects, renderer, filesystem, streams and
message queues remain foreign traits. No game files or analysis material were read.

Implemented city request/new/load/read/write/close and region-refresh algorithms,
inclusive edge/corner overlap, segment registration and cleanup, exact city-init
message ordering, directed deferred quit, save notifications, save-package ordering,
backup/replacement rollback, obliteration and terrain-picker/window handling. Scoped
helpers balance on every ordinary return, including unsupported save naming. Regional
arguments are retained before old owned references are released. Created and borrowed
references are distinguished, including the borrowed graphics-driver dependency.

Added raw 0x1734-byte preference persistence with the published key/version, live
language/IME save quirks and preservation of unchanged noncanonical flag bytes;
plugin enable filtering and reload continuation; native face facts and published app
getters; foreign framework-entry ordering; the instance restart poll; hardware-driver
description correction; the corrected 28-service order and nine v2 subscriptions;
shutdown unsubscriptions/cheat teardown; resource/INI summary algorithms and bounded
message pumps. The existing runner is extended through run_owned, retaining explicit
boundaries for genuinely unspecified app algorithms. No app-owned unknown body is
delegated to a fabricated host-success callback.

The module README has an Owner closure table for every numbered application section
and subsection, with code/test references and explicit question IDs. The supplied
application spec has no separately numbered conformance-test section; its worked
examples are mapped to tests, including resolution, touching-city reload, renamed
save/rollback and save-notification filename formatting.

## Validation and evidence

- Focused application suite: 48 passed (25 existing plus 23 new). The corrected final
  source was subsequently covered by the complete workspace run.
- Final-source workspace Cargo tests: **5954 passed, 0 failed**, summed over **27**
  `test result: ok` lines in `build-runs/gate2-qbuild-test.log`.
- Final-source workspace clippy with `-D warnings`: **passed**, finished in 31.56s;
  `build-runs/gate2-qbuild-clippy.log`.
- Explicit-file qbuild fmt: **6 files, exit 0, diff_lines 0**.
- Required qbuild all attempt 1: workspace tests completed; clippy found two fixture
  initializer warnings; CMake stopped progressing at compiler ABI detection. The
  operation was interrupted and terminal exit 1 collected. Full evidence retained
  in `build-runs/gate1-{qbuild-test,qbuild-clippy,fc4verify-configure}.log`.
- Fixed the actual lint diagnostics and the raw-preference review issue. Required
  qbuild all attempt 2 used `_CL_=/Z7` to test whether avoiding shared compiler PDBs
  resolved bootstrap. Cargo/clippy passed, but CMake again stopped at `Detecting C
  compiler ABI info`; CTest never started. The same operation was interrupted and
  terminal exit 1 collected. Evidence retained in corresponding `gate2-*.log` files.
  The PDB override did not establish a fix; the native gate is **incomplete**, not PASS.
- No duplicate build operations were launched for progress. No operations remain
  outstanding. No game requests: this task explicitly forbids plugin/capi wiring,
  and no native replacement or shadow was installed. Game verdict/ref warnings/shadow
  comparison are therefore not measured.
- q.py failed before execution with Git Bash CreateFileMapping access denied; focused
  tests used direct Cargo. A read-only CIM query during native-stall diagnosis was
  denied by Windows; no unrelated processes were terminated.

## Questions and remaining open sections

Appended **AP12, AP13, AP14, AP15** to `docs/specs/questions/application.md`.
The generated QUESTIONS index was not edited and forbidden tools were not run;
the coordinator must regenerate it.

Explicitly open: §§1-2 face identity/unknown interface slots (AP3/AP4/AP14/AP15),
§3 constructor/startup/version/user-data/OS/date/log/language/alpha/script details
(AP6), §3.1 config target/helper/resource details (AP5/AP6/AP13), §4 unnamed foreign
service identities (AP10), §5.1 exact heap-helper destruction discipline (AP6),
§5.2 service meaning (AP5, the two published calls are implemented), §6 GP apply and
preferences reapplication (AP9/AP14), §8 acceptance/numeric edges and opaque meanings
(AP8/AP11), §9 aliases/append ambiguity/slot 56 (AP3/AP7/AP15), §11.6 unpublished failure
and view defaults (AP15), §11.10 conflicting SavedCity summary (AP14), §11.15 INI path
source (AP13), §11.16 save-target naming/unnamed scope internals (§12/AP14), §13.3
video/reapply mappings (AP14; one-shot -x erasure implemented), and §14 full resource,
font/control/cursor/factory/INI resolution details (AP12/AP13). These are exposed as
explicit unsupported boundaries or explicit independently resolved inputs; the table
does not claim all 322 native functions or a complete executable application.

## Commits and next steps

- 44c88948: initial city and compatibility implementation.
- 6900d0db: integrated owner algorithms, closure table and focused coverage.
- 878210b6: corrected raw preference flag preservation and lint diagnostics.
- Next: rerun `python C:/Users/tlori/FreeCity4/.claude/lane-bin/qbuild.py --wt
  C:/Users/tlori/FreeCity4/.claude/worktrees/core-app5 all` after fixing the native
  compiler-bootstrap environment. Review the owner table and resolve the listed
  explicit spec gaps. Hold this lane from landing until the native gate completes.

Main checkout (lane tools only): C:/Users/tlori/FreeCity4.
Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-app5-r1-manifest.json.

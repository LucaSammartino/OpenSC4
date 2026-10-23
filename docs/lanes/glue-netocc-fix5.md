| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netocc-fix5: repair cached capture dependencies | Codex / glue-netocc-fix5 | ready: cargo 6653/0, clippy clean, CTest 69/69; run 334 clean, netocc 73/73 exact | Publish detached tip, land, and credit verified netocc coverage. |

## Changes and root cause

Merged the requested `glue-netocc-shadow5` branch in fd035d87, preserving both
advisor and netocc configuration and observer entries. The final repair is
21b0c98b7: an explicit local `FC4WriteCapture.h` dependency for plugin C++ sources.
The absent/off netocc ini key also returns before retaining the city or capturing
occupants; the old environment variable cannot activate it. Other shadows retain
their original variant rejection behavior. The imported regression shadows profile
still includes `netocc_shadow = 1`. No public C ABI exports or version changed.

The crash was stale compiled layout, not netocc static teardown. Ninja recorded
the cached `FC4NetMgrShadow.cpp.obj` dependency as
`../../../train-integration/engine/plugin/FC4WriteCapture.h`. A local header edit
therefore left that object unchanged. A debugger of our own test/DLL caught the
exit access violation in `WriteCapture::~WriteCapture`, destroying the net-manager
shadow's static `g_ourLot` with the wrong container offsets. No game executable
was inspected. Facts and the reproducer are in build-runs/exit-diagnostic.txt and
debug-native-exit.ps1.

The initial fresh merged-source verify passed 69/69. An experimental layout
isolation then reproduced the stale-object problem: 67/69, with exit AVs in
plugin_disable_test and plugin_ini_dialogs_test. That experiment was reverted
after identifying the dependency cause. Its failure log remains at
build-runs/failed-isolation1-ctest.log. WriteCapture itself now matches the imported
shadow branch. The final generated Ninja edge names this worktree's header
explicitly, and NetMgrShadow rebuilt at edge 129/203.

## Final validation

Stable engine source **21b0c98b7**, corrected-source `qbuild all`:
workspace cargo **6653 passed, 0 failed, 0 ignored** (36 result lines); clippy
**PASS, zero diagnostics**; native build **203/203 edges** (8.4 s); full CTest
**69/69 passed** (79.2 s). Native request
`qbuild-20261023-233345-build-ctest` was collected to terminal `qbuild: PASS`.
Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log and
fc4verify-ctest.log. Subsequent commits change documentation only.

## Real-city evidence

**`netocc-fix5-2032-21b0c98b7-1 72/72`** is the exact coverage line for dated
samples (2026-02-01 through 2032-01-01). Including load:
**`netocc-fix5-2032-21b0c98b7-1 73/73`**; shutdown reports **clean=73/73**.
Every sample has mismatches=0, records=1283, bytes=478251. Load fixups=0;
monthly fixups=857, using the imported NO45 canonicalizations.

Konradshohe, turbo=1, end date 2032-01-01; actual date **2032-01-01**,
real_seconds=100.188, verdict **clean**, ref_warnings=0, foreign_exceptions=0,
watchdog_stalls=0, rtc=0, overruns=0. DLL SHA256
`67b7a633b592055a2e2509c0ef39485a636e95597c5ee5610b88d08dcedaa017`.
The single game request was waited with a blocking filesystem watcher and
collected to its terminal verdict. No differences required a normal-speed follow-up.
This measures the fixture's sampled occupants, not absent occupant classes.

Run record: [netocc-fix5-2032-21b0c98b7-1-334.md](../runs/netocc-fix5-2032-21b0c98b7-1-334.md).
Raw evidence: build-runs/game-results/netocc-fix5-2032-21b0c98b7-1.log and .verdict;
build-runs/summarize-netocc-fix5.py reproduces both exact counts.

## Questions and next steps

No new spec questions. No native or game operation remains outstanding.
Supervisor publishes the detached commits; coordinator lands and credits coverage.
The explicit dependency guard covers WriteCapture; general remapping of cached
MSVC include paths for other headers remains a coordinator tooling follow-up.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-www5: web/network owner closure | Codex, core-www5 | ready | Coordinator review and merge; retain documented spec limits |

## Changes

Implemented the settled core contracts of CookieManager, WWWSystem, HTTPClient and
NetworkFileCache in the existing `engine/core/src/www` modules, reusing `rzurl`.
Filesystem, sockets, clock/calendar and native allocation identities remain foreign
traits. There is no plugin/capi wiring, ABI change or default replacement.

- Cookie parsing, admission/replacement, matching/header quirks, eviction, lifecycle,
  enumeration and the specified text persistence sequence (`cookies`).
- Cache defaults/settings, lifecycle, signed read expiry versus unsigned maintenance,
  RAM/file retrieval and holder ownership, destructive byte replacement, counters,
  materialization and the bounded INI writer (`cache`).
- HTTP synchronous borrowed notifications/progress and runtime calendar conversion
  (`notification`); existing admission/lifetime extended through worker exit (`http`).
- WWW constructor identity/component effects, FTP admission boundary and the outer
  HTTP admission helper held through WWW job insertion (`service`).
- Owner closure table, one row per assigned spec section, and a vector-to-test index
  in `engine/core/src/www/README.md`. CK01–06, WWW-01–20, HTTP01–09 and NFC01–12
  execute in `engine/conformance/tests/web_network_*.rs`; no live endpoints are used.

Implementation milestones: f4277b01 (owners), 77c7b8a2 (conformance relocation),
25d407e5 (constructor registration), 20754b82 (HTTP admission boundary ordering).

## Verification

Final stable engine source: 20754b82. One final `qbuild.py ... all`, collected through
its actual terminal result in session 9932: **PASS**.

| Check | Result | Evidence under `build-runs/` |
|---|---|---|
| Workspace cargo tests | 6,451 passed, 0 failed, 0 ignored (sum of all test results) | `qbuild-test.log` |
| Clippy, warnings denied | 0 diagnostics | `qbuild-clippy.log` |
| Native CMake build | PASS, 49/49 edges, 99.7 s | `fc4verify-build.log` |
| Full CTest | PASS, 69/69, 145.0 s | `fc4verify-ctest.log` |
| Native queue verdict | pass, total 248.7 s | `build-results/qbuild-20261023-224629-build-ctest.verdict` |
| Focused final WWW conformance | 12/12 passed | `web_network_www` cargo test |
| Changed Rust formatting | 2 files, zero diff | qbuild fmt; all 11 owned files passed in prior round |

The native tool reported “tree changed during the run”: only the lane handoff was
committed while it ran (`git diff --name-only 20754b82 ffdd99c4` is solely this file).
All engine sources and build inputs stayed stable. No duplicate gate was launched.
The previous outstanding request, `qbuild-20261023-213449-build-ctest`, was collected
first: pass, 220/220 build edges and 69/69 CTests. Its pre-relocation evidence is
distinct from the final run.

Game runs: none; no shadow/replacement key and no native integration in this task.
Core conformance establishes the supplied contracts, not native game equivalence.

## Questions and remaining spec limits

Q15 was appended in the first round to `docs/specs/questions/web-network.md`; no
additional question was needed in round 2. Literal choices are commented and tested.
The following limits prevent a complete native whole-owner equivalence claim:

| Spec sections | Remaining qualification |
|---|---|
| 3.2 | Q1 Port splitter convention; literal n−1 behavior implemented; Q15 blank/split/decimal edge choices |
| 3.4 | Q15 separator indexing when leading attributes are skipped; literal pair indices implemented |
| 3.6 | Q3 exact binary transform/character conversion; unqualified constructor path supplied as input; Q15 settings choices |
| 4.1, 4.8, 4.13 | Q7/Q9 residual inventory, FTP scheduler admission and unresolved support bodies; specified lifecycle/admission sequence implemented |
| 4.4, 4.14 | Q9 opaque Cookie-header record construction and request/first-record normalization |
| 4.7 | Q10 matched null-target HTTP completion suffix |
| 5.1 | Q13 wire/framing/redirect/retry/finalization/date grammar and residual native slots; Q9 normalization |
| 6.1–6.4 | Q11 INI-name literal, loader/escaping, unused-file selection, source-file admission, filename reservation and native buffer allocator |
| 6.4 | Q15 writer enumeration/materialization edge choices; direct byte-key order without read-side effects implemented |

Unsupported is explicit for unsettled owned bodies; it is never substituted with a
native false result. The settled remainder and all published constructed vectors
are implemented. No parked Q7/Q9 research round is reopened.

## Next

Coordinator: review the owner closure table, publish this detached HEAD to the
reserved lane branch, merge the ready queue entry and claim coverage after review.
Spec authors retain the questions above; this lane does not edit the coverage map.

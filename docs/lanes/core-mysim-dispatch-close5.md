| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-mysim-dispatch-close5: dispatch owner closure | Codex / core-mysim-dispatch-close5 | wip: native gate blocked | Resolve CMake/Ninja configure stall, rerun gate after environment correction; review MS19 |

## Report

Implemented the settled MS17 contracts for `cSC4MySimDispatch` and its
`cLocation` in the Rust core. The owner-closure tables in
`engine/core/src/mysims/README.md` map applicable specification sections to
code, tests, and the remaining questions. This is core implementation only;
there is no plugin/C ABI wiring, export change, or ABI-version change.

Changes:
- Added trip clock/wait timestamp, state/flag notifications and wrapping counters,
  queue copying/front/back/replacement, strict dwell expiration, recovery and
  destination selection, and the five signed tuning-field uses.
- Added GoTo acceptance/cancellation, commute request guards and hour/state
  branches, player entry/exit and held-automaton handling. Acceptance does not
  synthesize arrival. Pathfinder, traffic and transport remain foreign services.
- Added complete dispatch/location persistence, including opaque words, foreign
  references, route/map/queue and deferred setup; no invented RNG/padding slots.
- Extracted an in-memory dispatch fixture, added executable conformance coverage
  for all eight MS17 vectors' dispatch portions and a full opaque/reference
  roundtrip, and retained existing search/arithmetic vectors.
- Round 2 added explicit `DispatchHost::has_owner` and a centralized delivery
  gate for every notification path. Tests cover state/counters with no owner and
  owner removal between synchronous notifications. State locks are released
  before callbacks, and successive changed masks remain separately observable.

Milestones: `309413b1` implementation; `6fa120b4` conformance and owner tables;
`9eb1b47d` expanded semantic verification; `4e203f4b` first gate handoff;
`6aab950b` no-owner correction and its focused tests.

## Verification

- Final focused dispatch tests: **27 passed, 0 failed**. Four changed Rust files
  passed qbuild fmt with `diff_lines=0`; the earlier seven-file fmt also passed.
- Required final `qbuild all` on source commit `6aab950b` reached its actual
  terminal result: **FAIL native configure**. Workspace cargo: **5943 passed,
  0 failed, 0 ignored**, summed across all 28 `test result:` lines. Clippy:
  **PASS, 0 diagnostics** with warnings denied. Full CTest: **skipped, 0 run**
  because native configure did not complete. The nine dispatch conformance
  integration tests are included in the successful workspace run.
- Native failure is an infrastructure stall before any project C++ compilation:
  CMake stopped at `Detecting C compiler ABI info`. Its Ninja child had one
  waiting thread, zero recorded CPU time, no compiler child and no new output.
  Standalone Ninja `--version` and `-t commands` succeeded. After the repeated
  stall, stopped only this worktree's verified CMake/Ninja processes; collected
  the same qbuild session to exit 1. Native elapsed time reported: **312.4 s**.
  No duplicate final gate was launched.
- Final evidence: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, and
  `fc4verify-configure.log`. First-round incomplete evidence is preserved as
  `build-runs/r1-qbuild-test.log`, `r1-qbuild-clippy.log`, and
  `r1-fc4verify-configure.log`. Its old session was unavailable in round 2 and
  its original process had ended; it is not claimed as a passing gate.
- Earlier diagnostic failures and corrections are recorded in
  `build-runs/focused-results.md`. The record fixture's embedded-NUL GZStr was
  corrected; serializer semantics were not changed. The initial fmt invocation
  used relative paths and failed lookup; the absolute-path retry passed.
- Game runs: **none**; no game request outstanding. No native replacement or
  shadow comparison is claimed for this core-only work.

## Questions and remaining scope

Filed **MS19** in `docs/specs/questions/mysims.md`: equality precedence for
occupant/cell/neighbour identities (§4.6.1 and the §4.7.1 cross-reference), and
the effects/notice ordering of `TeleportToCommuteLocation` (§7.2.1). Equality
uses the explicitly commented literal `(occupant, cell)` comparison; state-1
DispatchCommute returns `UnsupportedMs19` without inventing teleport/arrival.

Existing **MS18** remains open for route-ready multimodal handoffs, final
proximity geometry and transport recovery/notice details (§4.6.1/§4.11),
dispatch-tool/player mission callbacks beyond settled entry/exit (§7.2.1),
and delegated transport RNG (§8). These boundaries return `UnsupportedMs18`;
there is no complete-trip success fallback. MySim-owned state/sound effects in
the commute conformance vector remain outside these dispatch owners.

All settled dispatch contracts are implemented and tested; complete owner
closure still depends on MS18/MS19. The merge queue is **wip**, held for native
verification. Next: correct the CMake/Ninja execution environment, rerun the
required gate after that diagnostic input change, then review the held entry.

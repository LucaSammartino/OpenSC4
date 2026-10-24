| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-nation10: cSC4Nation core | Codex / core-nation10 (detached HEAD) | wip: gates pass; filesystem owner closure blocked by NQ1 | Settle NQ1–NQ3, complete filesystem behavior, then add an economy shadow adapter. |

## Implemented

Module: `engine/core/src/nation/mod.rs`, with its owner closure table in
`engine/core/src/nation/README.md`. Implementation commit: `0920be534`.

- Application-owned catalog/directory getters, optional active index, and ordered
  old-region deactivation followed by new-region activation through an interface.
- Economy curve lookup and ID-0 fallback, wrapping month arithmetic with unsigned
  float conversion, float curve evaluation and city-only difficulty addition.
- Nation exemplar/property identifiers, plain property-index addition, duplicate
  demand-ID replacement, missing-data FreeCity4 defaults, empty curves and safe
  errors for unspecified original undefined reads.
- Shared thread-safe process tuning cache, retained through Shutdown/Init; all
  three subscriptions/unsubscriptions; city messages ignored and tutorial reset
  message routed to the reset method. Getters use no RefCell.
- Seven core tests and two nation §5 conformance tests, including a synthetic
  rising curve that detects the unsigned-date quirk and duplicate/empty tuning.

Nation §1 explicitly says there is no save record. No codec was invented; codec
round trips are inapplicable. Identical property inputs load deterministically.
No capi/plugin changes, no exported ABI changes, no ABI version bump.

## Verification

Final source was checked once with `qbuild.py --wt <WT> all` (session 69436,
collected to its terminal exit 0 and `qbuild: PASS`). Its native portion used the
coordinator's queued verification, job `qbuild-20261024-102200-build-ctest`; no
direct fc4verify invocation or duplicate native check was launched.

- Workspace Cargo tests: **7020 passed, 0 failed, 0 ignored**, summed by qbuild
  across test-result records; includes both new conformance vectors.
- Clippy with warnings denied: **PASS, 0 diagnostics**.
- Native build: **PASS, 255/255 edges**, 149.5 seconds.
- Full CTest: **PASS, 80/80**, 191.1 seconds; fc4verify PASS, 347.3 seconds.
- Owned-file formatting: **PASS**, 3 new Rust files, zero diff lines.
- Focused pre-gate Cargo run: **7/7** nation core tests pass.
- Diff whitespace check: PASS. Full gate logs remain in `build-runs/qbuild-test.log`,
  `qbuild-clippy.log`, `fc4verify-build.log`, and `fc4verify-ctest.log`.

The initial focused q.py wrapper failed before Cargo started (Bash
CreateFileMapping Win32 error 5). Evidence is retained in
`build-runs/q/20261024-101514-11952.log`; Cargo ran directly afterward and passed.
No source/test failure was hidden by a rerun.

## Open owner contracts

Questions are filed in `docs/specs/questions/nation.md`:

- **NQ1:** nation §1/§4 filesystem discovery/order, preferences, add/delete/copy,
  tutorial reset, lifecycle cleanup and failure/rollback contracts are absent.
  These remain open in §1, §3 (tutorial reset effects), and §4 (filesystem portion
  of Init/Shutdown and slots 10, 12–14). The core explicitly returns Unsupported
  for discovery/preference/add/delete/reset; reset-message dispatch propagates
  that error. This prevents claiming full owner closure or a ready lane.
- **NQ2:** §4 initial/invalid indices, same-index selection and region activation
  failures remain provisional. Supplied catalogs validate indices; failed index
  selection has no effects; same-index selection still deactivates/activates.
- **NQ3:** §2/demand §11.2 undefined missing-fallback and short-difficulty cases
  return explicit safe errors. Spec-defined missing-property defaults remain
  implemented; unsettled undefined cases need the spec writer's decision.

Only the allowed question file was edited under docs/specs. The coordinator must
regenerate the generated question index with its authorized tooling.

## Game and next steps

No game request was made and none is outstanding. This portable module is not
wired into the game, so there is no measured shadow or ref-warning verdict.
An economy-only in-game shadow is feasible next through game-interface date,
city-presence, difficulty and tuning inputs plus a capi/plugin comparison adapter.
Filesystem shadowing requires NQ1 to be settled first.

Next: obtain the NQ1–NQ3 contracts, replace the explicitly unsupported filesystem
methods, complete the open owner-table cells, and then wire the economy shadow.

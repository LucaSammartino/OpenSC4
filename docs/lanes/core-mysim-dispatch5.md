| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-mysim-dispatch5: dispatch and pool owner closure | Codex; core-mysim-dispatch5 (detached; supervisor publishes) | wip: pool closed; dispatch contracts implemented, full owner open under MS17; full gate PASS | Resolve MS17 before claiming complete dispatch owner coverage |

## Report

The pool's specified owner contract is closed. Dispatch location, persistent
job-search and state/flag notification contracts are implemented and covered.
Complete native dispatch ownership remains open under MS17; this lane makes
no claim that the out-of-scope automaton has been replaced. Per-owner section
tables are in `engine/core/src/mysims/README.md`.

### Changes

- MS13: replaced floor/saturating conversion with the specified float32
  magic-number and wrapping bit arithmetic, including tiny negative values,
  large coordinates, NaN payloads and infinities. Cell setters update world
  coordinates using the wrapping int32 shift and preserve other fields.
- MS14: missing gender/zodiac now skips a member; short listed names retain
  their full stem. Signed member/portrait formatting has directly testable
  helpers. Missing/bad counts stay zero per family; UINT_MAX preserves the
  specified nonterminating wrapping loop, never exercised as an import test.
- Core tests cover all published MS13 bit pairs, cell shifts and preservation,
  home occupant versus cell-lot rectangle fallback, required import reads,
  empty localized strings, short names and signed formatting.
- Added conformance vectors `ms12_29_location_arithmetic`,
  `ms12_30_import_branches`, `ms12_31_home_rect`; corrected existing neighbour
  world-position expectations and supplied required zodiac values in fixtures.
- Reused existing records/hosts. No street, plugin, capi, ABI or coverage-map
  edits. No native reference ownership or game services are fabricated in core.

### Validation

- Explicit fmt: PASS for the five owned Rust files.
- Focused core: 81 passed. Initial compile E0689 was fixed with a u32 counter.
  The next focused conformance run passed 30/31; its remaining stale far-edge
  world expectation was corrected from (-16,-16) to (1008,1008).
  Failed evidence remains in `build-runs/q/20261023-124950-32724.log`,
  `20261023-125104-38144.log`, and `20261023-125349-14852.log`.
- Required full `qbuild all`: PASS, collected original session 95083 to its
  terminal exit 0. Rust: 5866 passed, 0 failed, 0 ignored, summed across all
  27 test-result lines. Clippy: exit 0, 0 diagnostics, warnings denied.
  Native build: 217/217 edges; full CTest: 68/68, 0 failures (146.5 s).
  All three new vectors and both new core tests passed in this gate.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-build.log`, `fc4verify-ctest.log`. Native wrapper took 1156.2 s
  and reported a changed tree: `git diff --name-only 06d43729..183951f0`
  proves only this lane report changed; tested engine source remained stable.
  A private PDB-helper shutdown diagnostic rejected its unsupported switch;
  no server shutdown was established. The original gate completed normally.
- Game: n/a, no requests and no shadow claim; these core changes add no
  plugin/capi wiring to expose the new contracts in the real game.

### Questions and remaining owner scope

MS17 appended to `docs/specs/questions/mysims.md`. Exact open dispatch
sections: 1.1 (automaton scope), 3.3 (autonomous trip notification generation),
4.6 (current location/destination and wait evolution), 4.11 (tuning effects),
6.2 (dispatch record, including version-12 inline bytes), 7.2 (trip/tool/
U-Drive-It entry points), 8 (dispatch RNG), and 10 (stub/replacement boundary).
The spec explicitly places the automaton outside its scope and publishes no
dispatch record format. Those behaviors remain Unsupported, without invented
timing, randomness or bytes. No pool section remains open.

Next: spec owner supplies or identifies the missing dispatch contracts;
implement and test those before claiming full dispatch-owner closure. The
coordinator should regenerate the question index (engine lanes cannot run
the forbidden tools scripts). Queue is wip to prevent a complete-owner claim.

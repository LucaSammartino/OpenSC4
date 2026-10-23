| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-aircraft5 (airplane and helicopter owners) | Codex / detached `core-aircraft5` worktree | **WIP: owners not closed.** Cargo 6087/0, clippy clean, fmt 4/4; native configure incomplete, CTest skipped. ABI unchanged. AF3–AF8 filed | Answer missing aircraft contracts (AF1–AF8), complete owner laws, resolve native Ninja probe and rerun full gate |

## Result

The requested complete owner closure is **not achieved**. The implemented
contracts are tested and committed, but aircraft.md leaves the flight models
explicitly open and describes much of the remaining behavior only in summary.
No replacement control laws were invented or delegated to foreign managers.
This lane is queued **wip**, not ready to land as an owner-closure result.

## Implemented

- Added `engine/core/src/automata/aircraft.rs`: airplane/helicopter owners using
  `AutomatonBase` directly, without a traffic record or new `RefCell` getters.
  World surfaces, occupant targets, station properties and exemplar data stay
  foreign traits. No plugin/capi wiring, exports, ABI bump or manager changes.
- Extracted existing shared SetState/PushState/PopState contracts onto
  `AutomatonBase`; the generic wrapper delegates to the same implementation.
  Final-state refusal/DISAPPEAR insertion and stack indices remain shared.
- Airplane list/default loading, parent fallback, zero-derived entries and
  explicit speed scaling, with separately supplied AI/player lists.
- Nested aircraft/concrete save codecs, version checks, home placement,
  city-independent Read, atomic replacement on successful Read, first-Init
  home restoration and suppression of home writes after nested failure.
- Ground/water/building/prop altitude envelope and outward 1000-unit bounds.
- Independent flight phases and phase-1 airplane ammo refill without repeated
  refill; the specified LOITER timer/ammo facts and helicopter 6000-ms push.
- Plane CHASE rejection; helicopter null/non-player-target rejection, lead and
  hover geometry. Valid-target movement remains Unsupported(AF3). CROP_DUST
  remains name-only. Physics/rotor calls return Unsupported(AF2) without motion.
- Airplane/fire-plane effect resource names, as declarations rather than fake
  instantiated effects; foreign station available-unit property initialization.
- Added the README **Owner closure** matrix, one row per aircraft.md section
  for each owner, including §3 subsections and shared save/state contracts.

## Verification

`engine/conformance/tests/aircraft.rs`: **16 passed**, including all three
aircraft.md §8 cases, both concrete version gates, every truncated-read prefix,
failed nested writes, home placement, phases/ammo, bounds and chase rejection.
The existing automata suite also passed **32 tests** after the stack extraction.
Focused logs: `build-runs/focused-aircraft.log`, `focused-shared.log`.

Required full checks were each collected to their actual terminal result:

| Attempt | Source | Cargo totals over all result lines | Clippy | Native/CTest |
|---|---|---|---|---|
| 1 | d15d487a plus shared-helper formatting | 6087 passed, 0 failed, 0 ignored | Four approx_constant errors on required spec decimals | Configure probe stalled 533 s; scoped probe/child terminated; verifier FAIL, CTest skipped |
| 2 | 6124f6ad (final Rust source) | 6087 passed, 0 failed, 0 ignored; 30 result lines | PASS, exit 0, zero diagnostics | Same configure probe stalled 114.7 s; scoped probe/child terminated; verifier FAIL, CTest skipped |

The correction uses narrow `approx_constant` allowances for the specified
decimal defaults and their exact expected-values test; it does not substitute
π constants. Explicit-file formatting: **4/4 clean**; `git diff --check` clean.

Attempt 1 full logs were retained using apply_patch moves as
`build-runs/attempt1-qbuild-{test,clippy}.log` and
`attempt1-fc4verify-configure.log`; its outcome is `attempt1-verdict.txt`.
Attempt 2 full logs remain `build-runs/qbuild-{test,clippy}.log` and
`fc4verify-configure.log`; outcome is `attempt2-verdict.txt`.
Both native process trees were scoped to this lane's qbuild invocation:
verifier Python → CMake → Ninja, with no compiler child or progress beyond
`Detecting C compiler ABI info`. Only that configure probe and its child were
terminated, and no duplicate check was launched while either was outstanding.
The native root cause is unresolved; these are incomplete native checks, not
native passes. No permission elevation or modification of shared build inputs.

Game runs: **n/a**, no requests issued/outstanding. These are unwired core
owners, so there is no aircraft shadow/runtime behavior claimed and no game
verdict or reference-warning count to report.

## Open sections and questions

Appended **AF3–AF8** to `docs/specs/questions/aircraft.md`; AF1/AF2 already
existed. The generated question index was not edited and forbidden tooling was
not run; coordinator should regenerate it through the normal spec workflow.

| Section still open | Missing owner contract / question |
|---|---|
| §2 | Player-drive tick ordering/control behavior: AF5 |
| §3.1 | Exemplar property/parameter meanings and player selection: AF1; short/non-finite lists and parent-chain edge cases: AF7 |
| §3.2 | Constructor defaults beyond known data, probes, effect attachment/start/stop, Init/failure/Shutdown sequence: AF4 |
| §3.3 | Goal switches/control laws, takeoff/landing/approach, runway clearance/selection, taxi/stop/ray/altitude, fire handling and failure paths: AF3; dispatch transitions: AF6 |
| §3.4 | Forces/attitude/collisions/crash response: AF2; phase triggers and AEMS: AF4; player controls/bombs: AF5 |
| §4 | Rotor parameter mapping: AF1; NextFrame/attitude/motion: AF2; helicopter controls/chase distance/teleport rules: AF3; InitInstance/PhysicsInit/state AEMS: AF4; spotlight/player/missiles/UFO laser: AF5; messages/dispatch: AF6 |
| §5 (outside flight-owner scope) | Oversized available-unit masks: AF8; full station membership/numbering/removal stays with foreign sources |

AF3 also records the literal `*1000*dt` chase formula, LOITER strict `>2000`
boundary, loaded timer and unknown/empty-ammo cases. The code comments identify
literal readings; missing behavior is not presented as completed owner logic.

## Commits and next steps

- e6652bb7: initial measured handoff.
- f4f9ac4b: aircraft owner data/settled behavior, closure matrix, 14 tests.
- d15d487a: all §8 cases and nested write failure coverage (16 tests).
- 6124f6ad: narrow spec-decimal lint allowances and shared-helper formatting.
- Next: obtain AF1–AF8 answers, finish the missing owner laws and lifecycle,
  resolve the native compiler-probe stall, then run the full gate. Keep the
  merge queue entry WIP until both owner closure and native verification pass.

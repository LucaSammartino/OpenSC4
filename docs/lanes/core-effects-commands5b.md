| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-effects-commands5b: settled effects script commands | Codex / `core-effects-commands5b` | ready; Rust tests 6201/0, clippy clean, fmt 20 files clean; native gate: coordinator; ABI unchanged | Coordinator native build/CTest on final tip, then integration; EF15 residual, EFP7 and EF16 remain open |

## Changes

Implemented the settled effects-script contracts in the existing portable core
modules, without plugin or C ABI wiring. The owner-closure tables in
`engine/core/src/effects_parser/README.md` map each published owner/section to
its implementation and tests through effects §19.52 / EC276.

- Concrete token readers, error-prefix semantics, dispatch dictionaries,
  directory identity and explicit final-release ordering (effects-parser
  §§9–12; SW1–8, DI1–5, LT1–3 and DR1–6).
- Root identifier/resource/camera/priority/preview and visual-effect commands
  (EC1–50), definition enter/exit and effectBase (EC51–71), plus rotation
  instruction/store ordering (§13; RO1–6).
- Particle scalar/rate/collision/walk/appearance commands (EC72–119),
  dynamic-particle/sequence/shake/light children (EC120–160), and
  decal/resource/timed-effect commands (EC161–208).
- Warp/source/emit/force/tractor commands (EC209–271) and the published
  visual-effect/effect-group registration dictionaries (EC272–276).
  Typed timed-effect, wiggle and tractor payloads clone on inheritance and
  clear on description reset. EC57 retains inherited tractor end bits.

Renderer, sound, resource, conversion and generic command-library services
remain foreign traits. No lexer, runtime renderer or native parser execution
is claimed. Entry/exit methods complete before the generic caller changes
the block stack; failure-prefix tests retain completed mutations and calls.

Implementation milestones: `be2e2dbb` reader/dispatch/lifetime, `46e94dd6`
root commands, `ebd8bba9` definitions/children/rotation, `72c9268b` particle
commands, `4f911aa1` resources/decal/timed effects, `3ab5fb5e` motion/force and
final dictionaries, `223dc7f7` final Clippy fixture correction. Round 3
`6d372b83` made the README exception list consistent; no further settled
implementation gaps were found in this final review.

## Validation

Final stable source, round 3:

- `qbuild test`: **PASS**, 6,201 passed, 0 failed, 0 ignored. Full evidence:
  `build-runs/qbuild-test.log` (session 1052 collected to terminal).
- `qbuild clippy`: **PASS**, exit 0, zero diagnostics with warnings denied.
  Full evidence: `build-runs/qbuild-clippy.log` (session 2273 collected to terminal).
- `qbuild fmt`: **PASS**, all 20 owned parser Rust files, zero diff lines.
  Paths are relative to `engine/`, using forward slashes. Two earlier path
  invocation failures checked no source; recorded in
  `build-runs/fmt-r3-invocation-failures.txt`.
- Prior focused parser check: 79 passed, 0 failed. The earlier EC275 failure
  (78/79) is preserved in `build-runs/parser-check-ec275-failure.txt`; it led
  to EF16 and an explicitly bounded identity assertion, not an inferred fix.
- Prior full-gate Clippy failure is preserved in
  `build-runs/failed-final-gate-clippy-1.txt`; `223dc7f7` corrected its
  needless-range-loop fixture, validated by this round's passing Clippy gate.
- **Native gate: coordinator.** Per the round 3 instruction, no CMake/CTest
  was run or collected, including old session 82804. The prior incomplete
  native configure is not a passing gate. Coordinator must build and run
  native CTest on the queued tip.
- Game runs/shadow: n/a, portable core only; no game request outstanding.
  ABI exports and FC4_ABI_VERSION unchanged.

## Questions and remaining boundaries

- **EF15 residual**, `docs/specs/questions/effects.md`: unpublished execution
  for the §19.52 visual-effect nested families remains open. Their published
  registration behavior is implemented; execution is not inferred. The
  table identifies each residual family, including select/particleSequence
  block enter/exit and nested aliases. The pre-existing settled sequenceEffect
  execution remains implemented.
- **EFP7**, `docs/specs/questions/effects-parser.md`: numeric qualification
  remains open. Rotation uses a qualified foreign arithmetic backend;
  unsupported/unqualified numeric domains do not claim arbitrary x87 parity.
- **EF16**, filed by this lane in `docs/specs/questions/effects.md`: effects
  §19.52 assigns instance handler `00596980`, whereas effects-parser §6.1
  assigns `0059A730`. Both literal registration ledgers are retained. Only
  EC275's claim of equal root/nested handlers is open; the settled §19.4
  instance algorithm and other EC275 registration assertions are implemented.

No other settled command rows are left unsupported. Native verification and
integration are the coordinator's next steps; future spec answers are needed
to close only the boundaries above.

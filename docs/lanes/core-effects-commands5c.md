| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-effects-commands5c (effects parser owner closure) | Codex / `core-effects-commands5c` (detached worker HEAD) | ready; cargo 6454/0, Clippy clean, CTest 69/69; ABI unchanged | Coordinator reviews and publishes commits to reserved lane branch |

## Report

Closed every EF15/EF16/EFP7 open row in
`engine/core/src/effects_parser/README.md`. No published command-owner section
remains open. Core-only changes; no plugin, C API, ABI exports/version, generated
ABI entries or coverage-map changes. No new questions filed.

- EF15, effects §§19.53–56: named particle/dynamic/decal/visual references,
  auxiliary shake/flash/tint entries, chain replacement, anonymous sound/brush/
  scrubber (four aliases)/automata/camera payloads, and select/particleSequence
  hooks. Preserve validation order, raw names, insertion-before-options,
  failure-prefix writes, camera's overlapping byte and select's count/tail quirks.
- EF16, §19.52: effectGroup instance registration now equals the root instance
  descriptor and handler literally; existing instance execution tests retained.
- EFP7, effects-parser §14: canonical extended operands, PC24/53/64 and all four
  rounding modes; independent scale stores and original comparisons; separate
  probability multiply/add/store boundaries; nonfinite, signed-zero, overflow
  and gradual-underflow results. Reciprocals retain actual guard/sticky bits.
- Owner closure maps every answered row to code and tests through EC305/NQ17.
  Twelve new test groups cover all 29 new EC vectors and all 17 NQ vectors,
  plus failure prefixes and control-dependent arithmetic intermediates. Existing
  registration, reader, error, append, directory, lifetime and rotation tests
  were rechecked. The §17 runtime/vector coverage table remains explicit about
  runtime owners and shipped-data censuses outside command scope.

Final stable-source gate on `abcc5204`: **qbuild PASS**. Workspace Cargo
**6454 passed, 0 failed, 0 ignored**, including **91 effects_parser tests**;
Clippy **0 diagnostics** with `-D warnings`; native build **47/47 edges**
(65.5 s), full CTest **69/69** (132.7 s), native total 200.1 s. Owned-file fmt
passed for all eight edited Rust files with zero differences. Logs:
`build-runs/qbuild-{test,clippy}.log`, `build-runs/fc4verify-{build,ctest}.log`.

Earlier evidence is preserved: `build-runs/focused-1.txt` records the corrected
switch-marking assertion; `build-runs/gate-1/` holds complete logs of the first
full run (6454/0, two Clippy diagnostics, native 69/69, tree-changed annotation).
The Clippy fixes and reciprocal intermediate audit are in `abcc5204`.
Implementation and owner-audit milestones: `e8180169`, `7524993b`.

Game/shadow: **n/a**. No game request or replacement/shadow switch is introduced;
no native parser or live control-word parity is claimed. Numeric results
stipulate canonical operands and masked exceptions. Reserved/noncanonical
inputs have no qualified result; unmasked exception delivery and unqualified
rotation/transcendental domains remain foreign as the spec permits. These are
explicit qualification boundaries, not remaining EF15/EF16/EFP7 questions.

Next: coordinator reviews the owner closure and publishes the actual detached
HEAD to the reserved branch before integration. No game request is outstanding.

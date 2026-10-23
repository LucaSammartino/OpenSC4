| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-construction-ld5 (construction occupant closure specs) | Codex / re-construction-ld5 | ready | Implement LD35–LD38 against lot-developer §6.7.15–16 |

## Report

Resolved LD35–LD38 in lot-developer §6.7.15 and added 19 conformance vectors in §6.7.16.
The visual contract covers scaffold topology, height propagation, crane model/post geometry, inner
crane positions and swing, relighting, draw order and state restoration, effect parameters, prop
anchors, crew flags/direction, texture resources and unclipped rectangle subtraction.

Instruction checks corrected the previous interpolation inference: state 2 divides elapsed days by E.
The multiplier is float32 bits 0x3B03126F (nominal 500 ms/day); removal uses the quantized N·segH
span. Crew flags initialize false,roofFlag and the direction is normalized. The crew render gate is
kEffectPriorityLevel≥3. The six-by-five resources are FSH texture bindings, not sim grids; the
lot-records lifecycle note now agrees. Crane y starts at zero on successful creation and subsequently
tracks scaffold height plus the crane offset. RemoveRectangle does not clip crossing cuts.

Evidence: main-checkout export queries, targeted construction-class decompilation, instruction/byte
queries for numeric and layout rules, and read-only PE constant extraction. No REA query was needed.
The requested engine/core/src/lot_records/README.md was absent in both checkouts; the core-construction5
handoff and the four question statements supplied the closure boundary. No engine source was read.
The q.py Bash wrapper failed to start with CreateFileMapping error 5; native PowerShell was used.

Validation: `python tools/doccheck.py check` passed; question index regenerated. No cargo/clippy/ctest
or game run is applicable to this spec-only lane. No C ABI change or proposed ABI rows. No new questions;
LD35–LD38 moved to docs/specs/questions/answered/lot-developer.md. LD33–LD34 remain open and out of scope.

Implementation follow-up: replace the named visual Unsupported operations using the new contract and
recording-service conformance vectors. The lane does not claim implementation or in-game validation.

## Handoff
- Goal: answer LD35–LD38 in lot-developer §6.7 with conformance vectors; spec text only.
- Done: spec milestone committed 5f580c42; LD35–LD38 archived and doccheck passed.
- Source closure README requested by brief is absent in both worktree and main checkout; use the lane handoff's explicit Unsupported list and question statements.
- q.py cannot start Bash (CreateFileMapping error 5); use bounded native PowerShell commands.
- Next: coordinator can land this spec-only result; implementers can close the construction visuals.
- Facts: multiplier bits 0x3B03126F (nominal 500 ms/day); completion DOES divide by E; crew bytes false,roofFlag; direction normalized; 6×5 entries are FSH texture bindings; crew gate kEffectPriorityLevel≥3.
- No ABI changes or game requests. Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-construction-ld5-r1-manifest.json.

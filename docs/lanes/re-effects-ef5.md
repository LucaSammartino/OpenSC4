| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-effects-ef5: EF15, EF16 and EFP7 owner contracts | Codex / `re-effects-ef5` (detached worker HEAD) | ready; doccheck passed; ABI unchanged | Coordinator lands spec and routes contracts to core-effects-commands5b |

## Report

Answered EF15, EF16 and EFP7. Only spec, question archive/index and this lane
record changed. No engine, plugin, C API, ABI-table or coverage-map edits.

- EF15: effects.md §§19.53–19.56 publish named component references, auxiliary
  shake/light entries, chain replacement, anonymous sound/brush/scrubber/
  automata/camera payloads, aliases, and select/particleSequence enter/exit.
  The §19.52 descriptor inventory links every residual handler to its contract;
  the closure inventory links the previously published root/child families.
  Added **29** unexecuted command vectors, EC277–EC305. Literal quirks include
  select's positive-weight count and whole-list tail adjustment, camera's
  overlapping zoom/rotation byte, and anonymous collection insertion before
  common-option validation.
- EF16: descriptor 00A9FA08 has +0C=0059A730 at root and under effectGroup.
  00596980 is an instruction inside effectsResource exit, not an instance
  handler. Corrected §19.52; §19.4/EC275's equal-handler assertion stands.
- EFP7: effects-parser.md §14 specifies the caller's PC24/53/64 and all four
  RCs, raw ST0/store separation, probability intermediates, masked-exception
  nonfinite/overflow/underflow and signed-zero behavior. Added **17** independent
  numeric vectors NQ1–NQ17. No live process control word, token spelling or
  arbitrary rotation transcendental accuracy is claimed.
- Questions moved to answered/effects.md (EF15, EF16) and
  answered/effects-parser.md (EFP7); tools/questions.py index regenerated.
- Evidence: read-only re_query instruction bytes, exported class pseudocode,
  and original PE descriptor/constants. REA was deliberately unused because
  the brief says no binary is open. No REA error or fallback occurred.
- Boundary: the requested Owner closure table lives under engine/, which the
  brief forbids reading. Used docs/lanes/core-effects-commands5b.md's residual
  boundary list instead; all three identified questions are answered.

Validation: doccheck passed; whitespace check passed. Temporary numeric ledger
audit checks finite nearest-even stores and exact extended encodings only;
it is not a native x87/game capture. Cargo, Clippy and CTest n/a (spec-only).
Game/shadow n/a; all published vectors are unexecuted. No ABI proposals or game
requests outstanding. Ready for coordinator integration and implementation.

## Handoff
- Milestones: 8627cfed EF16; 8c0485e2 EF15/EFP7 publication and archives.
- Final audit adds camera slave, precise pair-store timing and consistency fixes.
- Next: coordinator lands actual detached HEAD and routes specs to blocked owner.
- Queue kind re per explicit brief; supervisor publishes reserved branch.

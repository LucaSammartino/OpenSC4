| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-ef30 — EF26 renderer codecs; kind docs | Codex / .claude/worktrees/re-ef30 / detached HEAD, reserved branch re-ef30 | ready; question import a75be6d90, spec e1819ff56; questions and whitespace checks PASS, fixture arithmetic 3/3; EF26 answered; ABI unchanged, no seams reserved; game/shadow n/a | 1. Land docs. 2. replace-effectsres28 implements nonempty records using effects.md §18.10, then validates in game. |

- Merged replace-effectsres28 with only EF26 imported; retained current engine tree and other lane records.
- Added effects.md §18.10: twelve typed codec contracts, receiver/argument order, stack cleanup, return channels, ownership and version selection.
- Added complete decal/dynamic wire tables, masks, list-count gates and unaligned payload framing.
- Corrected optional collection-camera serialization to a counted float curve followed by four floats.
- Corrected visual selection to version exactly 1 versus every other u16; documented old-reader retained fields.
- EF26-C1 gives an 86-byte decal decode, collection publication, source destruction and normalized write-back.
- Seven static conformance cases total; arithmetic checked for decal, dynamic and camera cases (3/3).
- Moved EF26 to docs/specs/questions/answered/effects.md and regenerated the question index; questions.py check PASS.
- git diff --check PASS. Cargo, clippy and ctest n/a for this docs-only change.
- Evidence: main-checkout export/decompilation with instruction-byte checks. REA was not needed.
- No ABI exports changed; no proposed ABI-row file. No game runs or shadow equivalence claimed.
- Remaining: engine implementation and native/game acceptance; these specs do not claim replacement credit.
- Queue: build-runs/merge-queue/re-ef30.json, kind docs, ready.

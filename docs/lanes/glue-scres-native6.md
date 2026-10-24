| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-scres-native6: natural resource parsers | Codex, `glue-scres-native6` (reserved branch, detached HEAD) | in progress | Apply SCR8/SCR9, gate, measure natural parser replay to 2032. |

## Handoff
- Goal: observe natural native INI/resource parser dispatch and replay through core sc_resources.
- Done: read SCR8/SCR9 section 10. Implemented startup-thread natural command-token capture,
  file registration and nested file-call observation with hardware entry breakpoints.
- Done: focused Rust comparison tests 5/5; first fmt diagnosed local layout only, fixed in place.
- Next: final fmt/full gate; collect queued native verification; real-city turbo run to 2032.
- SCR10 asks for missing physical ReadLine ABI, needed for complete file-parser replay.
- Measured scope planned: command-line tokenization before substitution; expanded/standalone
  dispatch separately counted, no macro/variable/custom-command state equivalence claim.
- Preserve constructed adapter coverage; default-off shadows; no public ABI changes planned.
- No game/build requests outstanding.

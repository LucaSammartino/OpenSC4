| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-scres-native6: natural resource parsers | Codex, `glue-scres-native6` (reserved branch, detached HEAD) | in progress | Apply SCR8/SCR9, gate, measure natural parser replay to 2032. |

## Handoff
- Goal: observe natural native INI/resource parser dispatch and replay through core sc_resources.
- Done: read SCR8/SCR9 section 10. Implemented startup-thread natural command-token capture,
  file registration and nested file-call observation with hardware entry breakpoints.
- Done: focused Rust comparison tests 5/5; first fmt diagnosed local layout only, fixed in place.
- Gate PASS: qbuild all workspace tests 6890 passed, 0 failed/ignored; clippy 0 diagnostics;
  fmt PASS. Native build 233/233 (85.3 s), CTest 72/72 (161.9 s), total 251.0 s.
- qbuild-20261024-032354-build-ctest collected to terminal; logs build-runs/fc4verify-{build,ctest}.log.
- Next: collect real-city turbo run scres6-native-2032-cobalt to 2032 (no other game request).
- SCR10 asks for missing physical ReadLine ABI, needed for complete file-parser replay.
- Measured scope planned: command-line tokenization before substitution; expanded/standalone
  dispatch separately counted, no macro/variable/custom-command state equivalence claim.
- Preserve constructed adapter coverage; default-off shadows; no public ABI changes planned.
- Game request scres6-native-2032-cobalt outstanding; no native build outstanding.

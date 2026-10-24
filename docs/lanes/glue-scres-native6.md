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
- Game scres6-native-2032-cobalt: clean to 2032-01-01, ref_warnings=0, scres 76/76;
  native 842 token comparisons, 1061 lines, 2 registrations/3 file calls; constructed 56/56.
- Run record docs/runs/scres6-native-2032-cobalt-386.md. No request outstanding.
- Diagnostic: shutdown summary mislabeled combined observations as adapter_reads.
  Corrected summary and added file/other route sample attribution (section 10 requires route separation).
- Corrected-source qbuild all: workspace tests 6890 passed, 0 failed/ignored; clippy 0 diagnostics.
  Native request qbuild-20261024-033751-build-ctest queued; collect session 64012 to terminal.
- Next: collect corrected-source native gate, then a unique corrected-source real-city run to 2032.
- SCR10 asks for missing physical ReadLine ABI, needed for complete file-parser replay.
- Measured scope planned: command-line tokenization before substitution; expanded/standalone
  dispatch separately counted, no macro/variable/custom-command state equivalence claim.
- Preserve constructed adapter coverage; default-off shadows; no public ABI changes planned.
- No game request outstanding; native request qbuild-20261024-033751-build-ctest outstanding.

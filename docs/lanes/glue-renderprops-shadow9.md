| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-renderprops-shadow9: render properties shadow | Codex, `glue-renderprops-shadow9` | in progress | Inspect public contracts, implement supported comparisons, gate and measure to 2032. |

## Handoff
- Goal: default-off renderprops_shadow; independent Rust expectations; real-city monthly comparisons to 2032.
- Done: default-off registry shadow implemented: 3 counts, 77 names, 462 core-derived Find probes (542 fields/sample), no native output seeds. RP7 filed for live setter inputs/view acquisition; no whole-owner claim.
- Focused gate: fmt PASS for capi/src/renderprops_shadow.rs; cargo test -p fc4-capi renderprops_shadow PASS 1/1 (579 filtered).
- Full gate terminal PASS: workspace cargo 6985/6985; clippy zero diagnostics; native build 254/254 edges (238.3 s), CTest 79/79 (224.0 s), qbuild PASS. Logs build-runs/qbuild-{test,clippy}.log and fc4verify-{build,ctest}.log. Queue label qbuild-20261024-083622-build-ctest. Only lane documentation changed during gate; source stable at aa6a2508b.
- Game r1 terminal CLEAN: renderprops-shadow9-20261024-r1 43/43 exact, 542 fields/sample, no duplicate months; ended at 2029-07-11 after 602.094 s, before 2032 target. foreign_exceptions=0, ref_warnings=0. Run record docs/runs/renderprops-shadow9-20261024-r1-469.md; unique evidence retained.
- Diagnostic: unknown-key warning despite active shadow; appended renderprops_shadow to FC4Config.cpp known-key list. This real source fix requires repeat all gate. No game outstanding.
- Graphics-rules spec sections 1/5/8 confirm startup/preferences ordered writes and persistence, but no read-only history/observation service; RP7 remains valid.
- Corrected-source all gate terminal PASS: cargo 6985/6985, clippy zero diagnostics, native build 95/95 edges (4.6 s), CTest 79/79 (96.7 s); qbuild PASS. Native queue label qbuild-20261024-085733-build-ctest. Final code stable at e1937e01f.
- Outstanding: renderprops-shadow9-20261024-r2 submitted; turbo house profile, seconds=1800 timeout=2100 end_date=2032-01-01. Do not submit any other request while pending.
- Next: wait with build-runs/wait-renderprops-verdict.ps1 -VerdictPath build-runs/game-results/renderprops-shadow9-20261024-r2.verdict; collect same exec session through terminal; repeat ONLY waiter after its 540 s timeout. Never resend r1 or r2; existing completed unique-label verdict is valid. Then summarize-renderprops.ps1 -Label renderprops-shadow9-20261024-r2.
- Initial fmt invocation used wrong relative path; retry with capi/src/renderprops_shadow.rs. q.py Bash failed CreateFileMapping (Win32 5); use PowerShell cargo directly.
- Constraints: no native signatures may be inferred; questions go in render-properties questions file. No DR registers without shared protocol.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-renderprops-shadow9: render properties shadow | Codex, `glue-renderprops-shadow9` | in progress | Inspect public contracts, implement supported comparisons, gate and measure to 2032. |

## Handoff
- Goal: default-off renderprops_shadow; independent Rust expectations; real-city monthly comparisons to 2032.
- Done: default-off registry shadow implemented: 3 counts, 77 names, 462 core-derived Find probes (542 fields/sample), no native output seeds. RP7 filed for live setter inputs/view acquisition; no whole-owner claim.
- Focused gate: fmt PASS for capi/src/renderprops_shadow.rs; cargo test -p fc4-capi renderprops_shadow PASS 1/1 (579 filtered).
- Full gate terminal PASS: workspace cargo 6985/6985; clippy zero diagnostics; native build 254/254 edges (238.3 s), CTest 79/79 (224.0 s), qbuild PASS. Logs build-runs/qbuild-{test,clippy}.log and fc4verify-{build,ctest}.log. Queue label qbuild-20261024-083622-build-ctest. Only lane documentation changed during gate; source stable at aa6a2508b.
- Outstanding: game request renderprops-shadow9-20261024-r1 submitted; turbo house switches through 2032-01-01, 600 s city budget/900 s timeout. Exactly one request; never resubmit this label.
- Next: collect SAME waiter exec session 28153 to terminal. Script build-runs/wait-renderprops-verdict.ps1 watches game-results/renderprops-shadow9-20261024-r1.verdict without qwait/polling. If context lost, completed verdict is valid; otherwise rerun only waiter, not request.
- Initial fmt invocation used wrong relative path; retry with capi/src/renderprops_shadow.rs. q.py Bash failed CreateFileMapping (Win32 5); use PowerShell cargo directly.
- Constraints: no native signatures may be inferred; questions go in render-properties questions file. No DR registers without shared protocol.

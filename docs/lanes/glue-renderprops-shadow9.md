| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-renderprops-shadow9: render properties shadow | Codex, `glue-renderprops-shadow9` | in progress | Inspect public contracts, implement supported comparisons, gate and measure to 2032. |

## Handoff
- Goal: default-off renderprops_shadow; independent Rust expectations; real-city monthly comparisons to 2032.
- Done: default-off registry shadow implemented: 3 counts, 77 names, 462 core-derived Find probes (542 fields/sample), no native output seeds. RP7 filed for live setter inputs/view acquisition; no whole-owner claim.
- Focused gate: fmt PASS for capi/src/renderprops_shadow.rs; cargo test -p fc4-capi renderprops_shadow PASS 1/1 (579 filtered).
- Full gate: workspace cargo PASS 6985/6985; clippy PASS, zero diagnostics. Same all operation queued native build/ctest as qbuild-20261024-083622-build-ctest; exec session 97133 remains outstanding. Do not submit duplicate verify.
- Next: collect session 97133 to terminal native verdict, then game house profile to 2032.
- Initial fmt invocation used wrong relative path; retry with capi/src/renderprops_shadow.rs. q.py Bash failed CreateFileMapping (Win32 5); use PowerShell cargo directly.
- Constraints: no native signatures may be inferred; questions go in render-properties questions file. No DR registers without shared protocol.
- No game request outstanding.

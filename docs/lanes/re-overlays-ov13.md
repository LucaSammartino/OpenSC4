| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-overlays-ov13 (overlay capture boundary and inactive headers) | Codex / `re-overlays-ov13` (RE lane) | in progress | Resolve OV13/OV14, conformance vectors, doccheck |

## Handoff

- Goal: close OV13 and OV14 for glue-overlays-shadow5 using export evidence, prose and vectors only.
- Done: `b04f4a0db` initial handoff; extended sections 2.6/3.8.1, added eight section 9.2 paper vectors, moved OV13/OV14 to answered and regenerated index.
- Gate: doccheck passed; initial diff check found a trailing blank line in the question file, now removed.
- Next: commit answer milestone, inspect final diff and finalize report/queue.
- Verified: constructor ends with null index pointers but never initializes +E4/+E8/+EC; Init calls BuildCellIndex only when dynamic byte is zero (0x0073CCF1-0x0073CCF8).
- Verified: terrain-view Init directly constructs all four managers then directly calls Init at 0x0075C6BC/C8/D7/E7. No observer registration interface is supplied by this owner.
- Owner lane file on this base and main has no Owner closure table or `open:` rows; its Coverage limits section names OV13/OV14 and was read first instead.
- Model pointees are already specified in lighting section 5.8: points stride 12, UV stride 8, transform byte +1 and float words +4 through +52. Capture original data; projection output is not an input.
- Facts: main is `C:/Users/tlori/FreeCity4`; no engine reads/edits; no ABI changes or game runs planned.
- q.py cannot launch Bash here (CreateFileMapping error 5); use bounded PowerShell output directly.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-overlays-ov13-r1-manifest.json`.

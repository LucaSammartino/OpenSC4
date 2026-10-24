| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-www-q19q20 (FTP initialization and HTTP worker/parser contracts) | Codex / `re-www-q19q20` | in progress | Resolve Q19/Q20 from export, add vectors, run doccheck and enqueue |

## Handoff

- Goal: answer web-network Q19 and Q20 completely; spec text only.
- Done: Q19 committed at 682962c9e; parent §4.13/4.15 and FTP-control §7.1, five vectors.
- Done: Q20 contracts written in parent §4.14/5.6, HTTP wire §1.1/1.2 and lifetime §2.1;
  26 proposed vectors, including malformed tokens, proxy endpoint and resource publication.
- Next: regenerate question index, commit Q20, run doccheck; finalize report and ready queue.
- Q19 evidence: worker 008170c6–008170d7 sets +0x1b0 only; closes 00811f00/00811f90 verified.
- HTTP evidence: status 00817f60 uses two string tokens; version 00818140 uses fixed positions;
  worker +0x14 stays request, +0x18 response published at 0081c5d3; proxy connects to origin.
- Budget status reached 101295/150000; stop new RE work and finish the committed closure/gate.
- Sources: `C:/Users/tlori/FreeCity4/tools/re_query.py`, `decomp/`, `ghidra/export/windows/` (read only).
- Do not read/edit engine, plugin, capi, coverage map, HANDOFF or main checkout files.
- REA has no binary; use export/decomp only. No game runs requested; no ABI changes.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-www-q19q20-r1-manifest.json`.

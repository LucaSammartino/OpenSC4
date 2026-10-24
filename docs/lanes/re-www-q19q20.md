| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-www-q19q20 (FTP initialization and HTTP worker/parser contracts) | Codex / `re-www-q19q20` | in progress | Resolve Q19/Q20 from export, add vectors, run doccheck and enqueue |

## Handoff

- Goal: answer web-network Q19 and Q20 completely; spec text only.
- Done: Q19 answered in parent §4.13/4.15 and FTP-control §7.1; five proposed vectors.
- Next: finish HTTP token parsing, origin/proxy destination and request/response job fields.
- Q19 evidence: worker 008170c6–008170d7 sets +0x1b0 only; closes 00811f00/00811f90 verified.
- HTTP evidence so far: status helper 00817f60 uses two string tokens, not literal HTTP grammar;
  version helper 00818140 uses fixed byte positions; worker +0x14 stays the request.
- Sources: `C:/Users/tlori/FreeCity4/tools/re_query.py`, `decomp/`, `ghidra/export/windows/` (read only).
- Do not read/edit engine, plugin, capi, coverage map, HANDOFF or main checkout files.
- REA has no binary; use export/decomp only. No game runs requested; no ABI changes.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-www-q19q20-r1-manifest.json`.

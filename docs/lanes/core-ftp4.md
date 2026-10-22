| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-ftp4: portable WWW and FTP published bodies | Codex, core-ftp4 | in progress | Baseline, published contracts, implementation and gate |

## Handoff
- Goal: implement published portable WWW/FTP C bodies; retain explicit P/O and unsafe-domain boundaries.
- Worktree: reserved detached HEAD; do not switch branches or change shared refs.
- Done: baseline all launched (session 51977, qbuild-test.log still empty at first inspection).
- Done: published section 4 and FTP/control/lifetime companions read; parser/path first slice implemented.
- Next: primitive I/O interfaces; FTP transfer/login/listing/worker state machine; quick gate.
- Parser native unsafe domains return explicit Unsupported, never fabricated output or native false.
- No game runs authorized or needed; no ABI/glue/default/coverage edits.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-ftp4-r1-manifest.json.

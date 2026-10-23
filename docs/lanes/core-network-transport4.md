| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-network-transport4: portable socket/stream owner | Codex / `core-network-transport4` (engine lane) | in progress | Implement §§1–6 bounded transport and literal fixtures; one stable full gate |

## Handoff
- Goal: first portable raw-operation socket/stream owner, no native adapter or NameLookup.
- Start HEAD: 15aced92f138f36741d1bba60490ec3dfdbd7917 (detached reserved worktree).
- Read: rules/lessons, lane format, network-transport §§1–6/8–9 and open questions Q11–14; no prior socket owner found in allowed production paths.
- Baseline omitted per task's explicit single stable canonical all instruction.
- Implemented `engine/core/src/network_transport/`: socket/stream lifecycle, raw16 address storage, error table, one-call transfers, readiness, explicit logical teardown and raw-only host boundary. Additive lib wiring only.
- Literal fixture expectations authored before production. 38 portable tests passed; quick core 3440/0, integration 21/0 + 13/0 + 19/0; native build still awaiting actual wrapper terminal (session 22211).
- Formatting: owned-file in-place rustfmt + qbuild fmt passed using absolute paths. q.py Bash startup denied; no shell/config workaround used.
- Next: obtain quick terminal, commit stable source, run ONE qbuild all and capture actual terminal/full ctest. Then final report/queue only if clean.
- Unsupported: nonzero connect scheduling, native address adapters, notifications/options/factories/NameLookup. Q11–14 and old web questions remain open; no new question needed for prescribed portable choices.
- No game requests; no ABI/plugin/CAPI changes permitted.

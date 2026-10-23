| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-propdev-pv14: prop developer owner closure | Codex / re-propdev-pv14 | in progress | Locate PV14 owner closure inputs, verify export, specify and validate. |

## Handoff

- Goal: answer PV14 for glue-weather-propdev-shadow5 with prose rules and conformance cases.
- Worktree reused at detached HEAD; no engine files read or edited.
- PV14 read from the explicitly requested owner worktree's question/report; absent in this checkout/main.
- Owner report has Findings/Next steps rather than an Owner closure table; PV14 is its sole stated blocker.
- Spec §5.1–§5.3 now defines guarded snapshots and real-return transcripts; §12.15–§12.22 has eight prose vectors.
- Source verified using export asm --bytes and bounded decomp reads; REA not used (no binary open).
- Next: regenerate/check questions, run doccheck, review fixtures and record final report/queue.
- Layout: 624 u32 words, pointer +0x9C0, remaining +0x9C4; total 0x9C8, cursor + remaining = 624.
- Native selector call/return: kept 0x006F4677/0x006F467A; create 0x006F1804/0x006F1807.
- Original CRT pre-draw: 0x009EFBA4, EAX runtime-data pointer, state +0x14.
- Source main: C:/Users/tlori/FreeCity4; export queries via tools/re_query.py.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-propdev-pv14-r1-manifest.json.

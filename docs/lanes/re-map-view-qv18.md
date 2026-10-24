| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-map-view-qv18 (map-view QV2/3/14 closure) | Codex / `re-map-view-qv18` (RE lane; detached HEAD) | in progress; merged qv17 `8f269ba2a`; ABI unchanged; game n/a | (1) Resolve QV2/QV3/QV14; (2) doccheck and enqueue. |

## Handoff
- Goal: merge re-map-view-qv17, close traffic volume QV2, panels/groups QV3 and ring precision QV14 with behavior and conformance vectors.
- Done: initialized handoff `944684bd4`; merged qv17 `8f269ba2a` (only conflict: generated QUESTIONS.md, resolved by questions.py index).
- Next: inspect question entries and map-view sections only as needed; query Update traffic path.
- Sources: read-only export/decomp under C:/Users/tlori/FreeCity4; no REA binary is open.
- QV2: predecessor located Update traffic path in cSC4WinMapView.c lines 1902–2360; source 76, shift-1 temporary grids, ferry view 0x54.
- QV3: predecessor located buttons/show/hide at lines 3219–3900; helper bodies may be in unnamed exports.
- QV14: 0x0079E130 ring helper uses inherited x87 control; CRT 0x009F6C30 sets PC=53. Need exact store/round schedule.
- Do not read/edit engine or edit plugin, capi, ABI tables or tools/coverage-map.tsv.
- No new question, ABI row or game request. Guard: C:/Users/tlori/FreeCity4/build-runs/run4-re-map-view-qv18-r1-manifest.json.

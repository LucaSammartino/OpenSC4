| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-map-view-qv18 (map-view QV2/3/14 closure) | Codex / `re-map-view-qv18` (RE lane; detached HEAD) | in progress; merged qv17 `8f269ba2a`; QV2 specified; ABI unchanged; game n/a | (1) Validate/commit QV2; (2) checkpoint partial closure; QV3/QV14 remain. |

## Handoff
- Goal: merge re-map-view-qv17, close traffic volume QV2, panels/groups QV3 and ring precision QV14 with behavior and conformance vectors.
- Done: initialized handoff `944684bd4`; merged qv17 `8f269ba2a` (only conflict: generated QUESTIONS.md, resolved by questions.py index).
- Done: QV2 §4.1 plus vectors 36–39; moved original entry to answered/map-view.md. No split.
- Next: regenerate index, doccheck, commit QV2; stop new investigation at the requested two-thirds checkpoint.
- Sources: read-only export/decomp under C:/Users/tlori/FreeCity4; no REA binary is open.
- QV2: tract size 1 is shift 0, not shift 1; three fresh cell-sized grids. Primary/premultiplied palette[255] overrides are +0x548/+0x948, not conversion factors (which are +0x958/+0x95C).
- QV3: predecessor located buttons/show/hide at lines 3219–3900; helper bodies may be in unnamed exports.
- QV14: 0x0079E130 ring helper uses inherited x87 control; CRT 0x009F6C30 sets PC=53. Need exact store/round schedule.
- Do not read/edit engine or edit plugin, capi, ABI tables or tools/coverage-map.tsv.
- No new question, ABI row or game request. Guard: C:/Users/tlori/FreeCity4/build-runs/run4-re-map-view-qv18-r1-manifest.json.

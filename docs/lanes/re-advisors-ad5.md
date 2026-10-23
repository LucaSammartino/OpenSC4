| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-advisors-ad5: advisor record and field contracts | Codex / `re-advisors-ad5` (RE lane) | in progress | Resolve AD17/AD18 and remaining open AD rows; doccheck. |

## Handoff

- Goal: specify AD17/AD18 and other open advisor questions without engine changes.
- Done: AD17/AD18 committed 0a63ac2c; AD11/AD16 rules, required cases and archive entries now written.
- Next: doccheck, questions check and review; final report and queue.
- AD11: static startup SKU-specific string empty; call getter, append relative advisors\; no runtime measurement.
- AD16: ToString uses strings/numeric %.14g; other types null, FromChar(null) empties; scratch cleared per field.
- Facts: CitySituationAdvice serializable table 00A84508 shares plain Write 00408090 / Read 004082F0, CLSID differs.
- AD18: preserve strict mismatches; separately labelled transition conformance expects flags OR 0x10.
- Shadow lane file in this checkout and reserved worktree has no Owner closure table/open: rows; read its AD17/AD18 report instead.
- Original material: main checkout absolute paths only; no REA binary available; use re_query.py and decomp.
- No game run requested; prior shadow run has 0/73 exact and is not validation of this lane.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-legacycontrol4: docs/specs/legacy-control.md for owner cGZLegacyControl (41 of 57 fns specified) | Claude / .claude/worktrees/re-legacycontrol4 | ready (docs only) | LC4: remaining 15 interpreters; LC3 serializer 0x00965F73; LC1 vtable 0x00AD5344; LC7 grid cell nesting |

## Report
- Round 1 (e2534f7b, 788a031c, 5d4b976b): layout, slots, lifecycle, vocabulary, SetParameters, common + Slider interpreters (38/57).
- Round 2 (28e58dc6, 0fda97d2, this commit): section 9.2 TreeView (first-pass style words; second pass image table
  name:{g,i}:(rect), root node words, `node` 15-field records, depth-first build from `Root`, six map parameters),
  9.3 FileBrowser (first pass only; sort/column/font/colour/alignment/grid/filter words with exact codes),
  9.4 image-key helper 0x0094C1DA (param 0xDAA3B860+n; LC6 answered DIFFERS), 9.5 Grid (second pass grid colours;
  first pass defaults, enablehdr table, 22 style bits, cell/col/row entries). Quirks Q5-Q7.
- Measured: 41 of 57 owner functions specified (was 38).
- Questions: LC6 answered; LC7 filed (grid cell flag nesting, fields 7-11). Open: LC1, LC3, LC4 (15 interpreters), LC5, LC7.
- Not done this round: LC3 serializer 0x00965F73 / GetScriptFragment, LC1 vtable 0x00AD5344 (context budget).
- REA unavailable (target_unavailable, no app open); used re_query.py export + decomp/. Helpers build-runs/slim.py, peek.py.
- No ABI rows, no engine/, no game runs.

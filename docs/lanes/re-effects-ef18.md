| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-effects-ef18 (EF18/EF19 contracts) | `re-effects-ef18` / `re-effects-ef18` (RE) | in progress | Resolve queue/bootstrap lifecycle and legacy/component transforms; doccheck. |

## Handoff
- Goal: answer EF18 and EF19 in effects.md; archive questions and add prose conformance vectors.
- Done: `88f1af3c5` adds §3.1 legacy decoding, component arithmetic/store order, six proposed EF19 vectors.
- Done: `ea548753c` adds §§6.2/18.6 and archives EF18/EF19; questions index/check pass.
- Validation: initial doccheck check passed; final review adds optional-file exception behavior and camera-copy detail.
- Next: rerun doccheck/questions checks, commit final corrections, write Report and queue entry.
- Facts: ParseQueuedFiles calls file-parser ParseFiles, reports error or success; it is not a separate parser class.
- Facts: legacy placement is 13 floats plus a u32 revision; flags become 5 or 7; dot order z,x,y.
- No engine, plugin, capi or ABI edits permitted. Use main checkout export read-only.
- Source date is 2026-10-24; brief date is 2026-10-23.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-effects-ef18-r1-manifest.json.

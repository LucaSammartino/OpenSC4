| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-overlays-ov12: native record observation and replay contract | `Codex` / `re-overlays-ov12` (RE, detached worktree) | in progress | Resolve OV12; doccheck; publish report and queue |

## Handoff

- Goal: resolve glue-overlays-shadow5 OV12 with evidence, spec prose and conformance cases; no engine reads/edits.
- Done: read shadow5 report and its OV12 entry through git show. Local question file lacks OV12; preserve its wording in the answered archive when resolved.
- Facts: shadow5 report has no Owner closure table/open rows. Its explicit blockers are arena enumeration, record fields, cell/texture observation and independent replay inputs. Getter subset measured 73/73; no live-record credit.
- Next: query constructor/allocation/free routines and inspect owner decomp for all three arena headers and complete record fields.
- Sources: main C:/Users/tlori/FreeCity4; export via tools/re_query.py; decomp read-only. Manifest C:/Users/tlori/FreeCity4/build-runs/run4-re-overlays-ov12-r1-manifest.json.

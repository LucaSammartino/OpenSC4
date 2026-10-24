| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-overlays-ov12: native record observation and replay contract | `Codex` / `re-overlays-ov12` (RE, detached worktree) | in progress | Resolve OV12; doccheck; publish report and queue |

## Handoff

- Goal: resolve glue-overlays-shadow5 OV12 with evidence, spec prose and conformance cases; no engine reads/edits.
- Done: added §§2.4–2.7/3.8 and twelve §9.1 paper vectors; preserved shadow5 OV12 verbatim in answered archive (local base lacks the open entry).
- Facts: shadow5 report has no Owner closure table/open rows. Its explicit blockers are arena enumeration, record fields, cell/texture observation and independent replay inputs. Getter subset measured 73/73; no live-record credit.
- Next: verify citations and metadata against assembly; regenerate question index, run doccheck, then final report/queue.
- Findings: arena headers 98/AC/C0, strides B4/30/24, each five words begin/end/cap/free-head/free-tail; live −1, free tail 0. Index F0/F4/F8 holds list heads, nodes next/id. Model creation inputs and upload bytes are not retained; snapshot-only full replay is impossible.
- Sources: main C:/Users/tlori/FreeCity4; export via tools/re_query.py; decomp read-only. Manifest C:/Users/tlori/FreeCity4/build-runs/run4-re-overlays-ov12-r1-manifest.json.

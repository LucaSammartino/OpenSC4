| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-overlays-ov12: native record observation and replay contract | `Codex` / `re-overlays-ov12` (RE, detached worktree) | in progress | Resolve OV12; doccheck; publish report and queue |

## Handoff

- Goal: resolve glue-overlays-shadow5 OV12 with evidence, spec prose and conformance cases; no engine reads/edits.
- Done: `afc2794d4` investigation handoff; `8dbb458c7` §§2.4–2.7/3.8, twelve §9.1 paper vectors, OV12 answered archive and generated index.
- Facts: shadow5 report has no Owner closure table/open rows. Its explicit blockers are arena enumeration, record fields, cell/texture observation and independent replay inputs. Getter subset measured 73/73; no live-record credit.
- Done: direct `python tools/doccheck.py check` passed. q.py wrapper attempt failed before Python due to Bash CreateFileMapping access denial; direct PowerShell invocation corrected the launch, not source behavior.
- Verification: assembly confirms factory upload driver slot 61 (+F4), metadata field widths and arena relocation pointers; corrected draft slot/citation before final check.
- Next: run final doccheck/questions check, commit verification milestone; finalize report and queue with full HEAD.
- Findings: arena headers 98/AC/C0, strides B4/30/24, each five words begin/end/cap/free-head/free-tail; live −1, free tail 0. Index F0/F4/F8 holds list heads, nodes next/id. Model creation inputs and upload bytes are not retained; snapshot-only full replay is impossible.
- Sources: main C:/Users/tlori/FreeCity4; export via tools/re_query.py; decomp read-only. Manifest C:/Users/tlori/FreeCity4/build-runs/run4-re-overlays-ov12-r1-manifest.json.

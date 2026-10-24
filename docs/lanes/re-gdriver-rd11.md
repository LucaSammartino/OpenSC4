| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gdriver-rd11: DX7 RD6–RD11 closure | Codex / `re-gdriver-rd11` | in progress | Verify export behavior, extend spec and vectors, archive answers, doccheck. |

## Handoff

- Goal: answer RD6–RD11 in renderer-driver.md; spec text only, no engine reads/edits.
- Done: read rules, lane format, open questions and core-gdriver5 report.
- Owner closure table is in engine/, which this task forbids reading; use the report's explicit RD6–RD11 exclusions and the questions instead.
- Done: RD10/RD11 rules added: white missing colour; untouched missing UV; incomplete quad overrun; final draw HRESULT; ambient x87 CW and finite discriminator.
- Done: `0d5ac1e1d` establishes RD10/RD11 rules. RD8/RD9 prose added: first depth descriptor, selector quirks, setup errors; zero material/clear defaults, stage constructor versus dirty-only reset, list helper and destructor AddRef quirk.
- Done: `16da6b791` RD8/RD9 milestone; corrected static GetDriverInfo pointer and destructor details after instruction checks. Archived RD8–RD11 and added DX7-19–31.
- Next: finish RD6/RD7, reconcile §16 and inventory boundaries, then final report/queue. RD6/RD7 still open; no ready claim.
- RD7 raw auxiliary tables: owner+0=00AC47EC (13 words read; first 11 public plus destruction entries), +4=00AC47DC (QI/ref/ref/CopyColorBuffer), +8=00AC47AC (12 slots: QI/ref/ref, 883F30/883FA0/884000/886CA0/886B90/noop/noop/884200/8840D0).
- RE helper build-runs/inspect-rd.py reads PE constants; all edits via apply_patch. Guard counts context, not just this response's visible tokens; no queue until six answers complete.
- Worktree reserved with detached HEAD; commit normally without changing refs.
- Main RE source: C:/Users/tlori/FreeCity4; manifest: build-runs/run4-re-gdriver-rd11-r1-manifest.json there.
- No game runs, ABI rows or implementation changes planned.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audio-au50 (audio owner contracts) | Codex / `re-audio-au50` (RE) | in progress; AU45-AU49 and AA14 answered; doccheck passed | Audit contradictions; commit report; enqueue |

## Handoff

- Goal: close core-audio-close11's six questions with prose and conformance cases.
- Done: debbb740c AU45; 21399ceb3 AU47 archive; 6fe5ae5b5 AU46/AU49 contracts.
- Current: AU48 audio.md §10.7.1; AA14 audio-ambience.md §8.1.1/8.3 written and archived.
- All questions sourced from core-audio-close11 via git show; absent from this branch's open files.
- questions.py index regenerated docs/specs/QUESTIONS.md; doccheck.py check passed 2026-10-24.
- Next: audit earlier summaries for contradictions; commit report; create ready merge queue.
- Evidence: main re_query.py and class decomp files; no REA session used.
- Numeric rules checked against asm --bytes; PE constants read by ignored build-runs helper.
- AEMS AddBank zero means success; callback4E2410 identity; bank+20 stores zero on success.
- Two GetSize calls unchecked; TGI copied after Read success; resource result is Close result.
- Layer first failed Init retains latch/count; repeated Init true without retry; Shutdown balances.
- Layer singular loop removal no stop; singular freshness removal has extra Release.
- SoundManager DoEvent focus/stop-all returns false after effects; public wrappers differ.
- No engine, ABI, tools or game changes; no game requests.

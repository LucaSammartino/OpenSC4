| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-answers35 (RE answers: mysims questions MS1-MS7, docs only) | `re-answers35` / `C:\Users\tlori\FreeCity4\.claude\worktrees\re-answers35` (branch `re-answers35`, RE lane) | **in progress** — worktree created at main 7e042544; spec + questions read; starting RE. | Answer MS1-MS7 from the export with re_query.py; update docs/specs/mysims.md; move answered to docs/specs/questions/answered/mysims.md; questions.py index; lane report + merge-queue/re-answers35.json (kind docs, abi false). |

## Handoff

- Goal: answer the seven open mysims questions MS1-MS7 in docs/specs/questions/mysims.md by RE-ing the
  Windows export (re_query.py + ABI tables), checking numeric claims against instruction bytes; update
  docs/specs/mysims.md in place; move answered questions to answered/mysims.md; no engine/, no game runs.
- Method (re-answers34 shape): read each question, RE it, verdict MATCHES/DIFFERS where the spec states behaviour.
- Facts: worktree C:/Users/tlori/FreeCity4/.claude/worktrees/re-answers35 (branch re-answers35, base main 7e042544);
  MAIN = C:/Users/tlori/FreeCity4. Questions file: docs/specs/questions/mysims.md (MS1-MS10 open; MS8-MS10 are
  cores28's, NOT in scope — answer only MS1-MS7). Spec: docs/specs/mysims.md (62 KB, read by sections).
- Done: MS1 eecb2081, MS2 77929a0c, MS3 758a3a4b, MS4 78863510, MS5 (this commit). Helpers: build-runs/move_q.py
  (moves entry + answer file), build-runs/rd.py (read u32 at VA), build-runs/mysim_asm.txt (all cSC4MySim asm).
- Frames: Write/Read this = MySim+0xC, DoMessage = +0x10, Init/Shutdown = +0x8; FindNewJob/ctor/simulate_day = +0.
- Next: MS6 (CheckCommute 0x0042D9E0 / FindNewJob pathfinder side effects), MS7 (AnnounceMoveIn 0x0042BF70, states 4/5/6).

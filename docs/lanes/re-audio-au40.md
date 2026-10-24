| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audio-au40 (AU40–AU44 owner contracts) | Codex / `re-audio-au40` | in progress | Resolve owner gaps from export; doccheck and enqueue |

## Handoff
- Goal: answer AU40–AU44 with prose contracts and conformance vectors; no engine changes.
- Worktree reused at detached HEAD; main checkout is read-only evidence.
- Done: AU40 geometry/tuning/notification/failure contracts and ten spec vectors; question moved.
- Done: AU44 five views, defaults, fire filtering, dispatch, group lifetime and eight vectors.
- AU40 milestone commit: 1a339eeb2.
- Next: doccheck, finalize partial closure report and enqueue; AU41–AU43 remain open.
- q.py gate launcher failed before doccheck (Git Bash CreateFileMapping error); use foreground
  Python subprocess wrapper build-runs/check-docs.py, capturing build-runs/doccheck-re-audio-au40.log.
- AU40: notification compares/saves camera position; first two array fallbacks do not resize;
  null server/message allocations fault. All numeric rules checked against instruction bytes.
- Evidence: local re_query/export only; REA has no binary open.
- Questions live at docs/specs/questions/audio.md (brief's doubled paths are erroneous).

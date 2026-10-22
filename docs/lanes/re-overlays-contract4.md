| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-overlays-contract4: native overlay owner contracts | Codex / `re-overlays-contract4` (RE docs) | in progress | 1. Prove OV1 placement and publish vectors. 2. Resolve OV4-OV9 in dependency order. |

## Handoff
- Goal: native nine-float placement equations first, then constructor/handles/alpha/matrices/draw/round contracts.
- Worktree reserved, detached HEAD; main..HEAD empty and working tree initially clean.
- Done: first exact record/zero-angle placement, position tolerance and alpha contracts/vectors published before call 30; OV6 answered.
- Next: finish ordered rotated placement, then constructor/getters/handles; cell and round owner proofs follow.
- Read overlay spec sections 2/3/4 and open OV1-OV9; OV2/OV3 outside priority unless needed.
- q.py cannot launch Bash (E_ACCESSDENIED); use capped native PowerShell and re_query.py directly.
- Edit docs only; never read engine; ABI proposals only under build-runs/pending-abi, with exact rows also in abi-rows file.
- No game requests or runtime gates required for docs-only work; no verdicts claimed.
- REA batch (PlaceOverlay/ComputeCellRect/SplitDiscIntoParts/AddDecalFromBinding) failed target_unavailable; export/bytes fallback used.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-overlays-contract4-r1-manifest.json.

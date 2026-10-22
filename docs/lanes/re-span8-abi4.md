| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span8-abi4 (SR9 native ABI evidence) | `re-span8-abi4` / RE, detached worktree | in progress | Verify native contracts and publish bounded proposal. |

## Handoff
- Goal: resolve SR9 from native bytes; propose only necessary exact-key ABI rows, no ABI table or engine edits.
- Done: verified six renderer entries/pops (slot 4 tail jump); setter proof and draw-system slot 4 confirmed from export bytes.
- Done: spec now states byte-backed slot-22 void contract and six cleanup exits; triangle caller ignores its result.
- Done: native table reads and exact-key reconciliation find 37 scoped keys absent; prior proposal overlaps 14 (slot22 unknown superseded by proof).
- Next: finish native byte cross-check, buffer allocation/free lifecycle closure, bounded TSV, SR9 archive and validation.
- Proposal destination: `build-runs/abi-rows-re-span8-abi4.tsv` (also preserve reviewable evidence in tracked docs).
- Main: `C:/Users/tlori/FreeCity4`; worktree: `C:/Users/tlori/FreeCity4/.claude/worktrees/re-span8-abi4`.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-span8-abi4-r1-manifest.json`.
- q.py cannot launch Bash (CreateInstance E_ACCESSDENIED); use capped native PowerShell commands.
- Scratch audit: `build-runs/audit-re-span8-abi4.py`; native SHA256 `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
- Constraints: no engine/tools/ABI-table/unreconciled edits, subagents, branch switches, merge or push; final third family round.

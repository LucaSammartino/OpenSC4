| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span8-abi4 (SR9 native ABI evidence) | `re-span8-abi4` / RE, detached worktree | in progress | Verify native contracts and publish bounded proposal. |

## Handoff
- Goal: resolve SR9 from native bytes; propose only necessary exact-key ABI rows, no ABI table or engine edits.
- Done: `ff5283b8` verified six renderer entries/pops (slot 4 tail jump), setters and draw-system slot 4.
- Done: full native cross-check matches 1,782 instructions / 43 function ranges. Slot22 has eight RET8 exits, void; three rasterizer callers ignore EAX.
- Done: 40-row bounded proposal published (6 renderer,13 context,1 draw system,20 buffer), all exact keys absent on main `47efb91e`; prior proposal overlaps14 (13 matching, slot22 strengthened).
- Done: spec10.7 publishes full boundary, lifecycle closure and reconciliation; SR9 archived, duplicate open SR10 retired preserving coordinator clarification. questions index/check and whitespace pass.
- Next: final review of scoped diff and proposal schema, then final report/queue. Coordinator must apply reviewed absent keys through gated train; no bridge enablement from proposal alone.
- Proposal destination: `build-runs/abi-rows-re-span8-abi4.tsv` (also preserve reviewable evidence in tracked docs).
- Main: `C:/Users/tlori/FreeCity4`; worktree: `C:/Users/tlori/FreeCity4/.claude/worktrees/re-span8-abi4`.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-span8-abi4-r1-manifest.json`.
- q.py cannot launch Bash (CreateInstance E_ACCESSDENIED); use capped native PowerShell commands.
- Scratch audit: `build-runs/audit-re-span8-abi4.py`; native SHA256 `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
- TSV SHA256 `0641b79e9fc283434af58419724c4306a98361e4ce4f943d0e8185f506107165` (12-column existing schema; no unknown pops/returns).
- No native ABI gaps in bounded route. Remaining SR8-2,SR8-3,SR8-4 are unchanged; no game runs, fixture execution or compiled gate (docs-only).
- Factory 0082B910 is not decoded; capstone fallback unavailable. Existing factory proof retained; no new factory row. No REA calls.
- Constraints: no engine/tools/ABI-table/unreconciled edits, subagents, branch switches, merge or push; final third family round.

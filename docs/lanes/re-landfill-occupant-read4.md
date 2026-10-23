| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-landfill-occupant-read4: occupant nested read and position return contracts | Codex / `re-landfill-occupant-read4` (RE docs lane; detached HEAD) | in progress; no game runs or ABI changes | 1. Trace LG33 reader layers and LG32 return channel. 2. Publish evidence and check docs. |

## Handoff

- Goal: resolve LG33 nested-holder acceptance/stream/state contract and LG32 inherited SetPosition return separately from settled effects.
- Done: initial handoff committed as `7a081829`; reader layers and position body instruction-checked; three existing specs clarified, literal vector companion authored, LG32/LG33 moved with individual verdicts.
- Next: review final diff and commit evidence milestone; finish lane report/queue.
- Scope: documentation only; no engine access, native replay, expected-value changes, LG31 investigation or shared ABI edits.
- Main sources: `C:/Users/tlori/FreeCity4`; local scratch/evidence: `build-runs/`.
- q.py Bash wrapper fails with `Bash/Service/CreateInstance/E_ACCESSDENIED`; use capped PowerShell reads and Python queries directly.
- REA unavailable: `target_unavailable`, “No app is open.” No opening/retry loop or target execution; export asm --bytes sufficient.
- Facts: base ignores exemplar AL at 005E8DD8; exemplar tests plain AL at 005FF454; landfill tests base AL at 0047758D. Rejected exemplar v1 resumes at 18 and consumes tail through 39, error 0.
- Position slot 9 target 00477600 leaves EAX=z bits; input z=120.25 gives EAX=42F08000, AL=00. Concrete void/effects contract only.
- Proposed short-read vectors deliberately leave stack-temporary/count effects partial; no new native runs, historical LG33 DIFF preserved, LG31 excluded.
- Checks: questions index regenerated, questions check passed; literal/float/misalignment/local link-fence checks passed; initial diffcheck found only open-question EOF blank, removed before final rerun.
- Pending ABI annotation only: `build-runs/abi-rows-re-landfill-occupant-read4.tsv` (existing vtables.tsv format, one SetPosition row; shared tables unchanged).
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-landfill-occupant-read4-r1-manifest.json`.

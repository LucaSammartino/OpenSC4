| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-tools-director4: cGZToolsCOMDirector native contract | `re-tools-director4` / (RE spec lane, detached worktree) | in progress; inventory discovery; no ABI or game runs | 1. Complete attributed-body inventory. 2. Specify registrations and lifecycle. 3. Record exact gaps and discriminating vectors. |

## Handoff
- Goal: specify only cGZToolsCOMDirector; `31293B` is the attributed byte count, not a class-name suffix.
- Done: read application lane report; cSC4App and automata director are outside scope. Clean detached worktree reused.
- Done: current attribution exact inventory 70 bodies / 31,293 B; committed spec includes every row and substantive director lifecycle, registrations, QI/refcounts, lookup/enumeration/destruction, 12 independent vectors.
- Director tables 00AD4F60 primary and 00AD4F34 hooks; all 9 lifecycle hooks return true with no side effects.
- Registrations 3CEC2B47 -> 0094AB94 (1104 bytes); 9CEC2B58 -> 0094A4A6 (12 bytes). Most attributed bytes are reached decoders, not director methods.
- Next: trace registered-service largest bodies and native boundaries, then narrow TD1/TD2/TD3. Whole-owner credit explicitly withheld.
- Questions filed docs/specs/questions/tools-com-director.md (TD1-TD3). No answered entries yet.
- Native sources: C:/Users/tlori/FreeCity4/{bin/SimCity4.exe,decomp,ghidra/export/windows,re}; edits only here.
- q.py Bash wrapper fails with E_ACCESSDENIED; use local PowerShell with capped output. qgrep needs Python UTF-8 for non-ASCII exports.
- No implementation/test evidence, engine reads, ABI edits, game requests, external communications or subagents.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-tools-director4-r1-manifest.json.

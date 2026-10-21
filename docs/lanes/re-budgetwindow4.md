| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-budgetwindow4: resolve budget-window.md questions BW1-BW9 (RE spec lane) | fc4-lane agent, .claude/worktrees/re-budgetwindow4 | ready: BW1-BW4, BW6-BW9 answered and archived; BW5 open (game check) | core lane: reconcile the 21-test core with the DIFFERS rows (scroll bound, no button tail, f32 product, f32 levels, Release/pause rules, returns); BW5 needs a game run |

## Report (final, round 3/3)

Spec-only lane (docs/specs). No engine, tools or ABI edits; no game runs.

Commits:
- 7856795d: §3 (`+0x24` is a department ID), §4.1 group lists A/B and special-group headers (BW1), §4.4 f32 slider
  product (BW8), §5 table (no refresh tail), §5.3 scrolling (BW7), §5.4 ApplyDealEdits (BW2), §5.5
  SetOccupantFundingLevel (BW3), §5.6 popup child, §5.7 camera jump.
- 4ec53fde: BW1/BW2/BW3/BW7/BW8 moved to docs/specs/questions/answered/budget-window.md (new file), index rebuilt.
- 1fbc4404: BW4, BW6, BW9 answered and archived; §2 references + nested individual dialog, §2.1 tab byte `+0x194`,
  §4 DoWinMsg returns / on_init guard / on_shutdown, §6 end_dialog code argument, pause counter balance, §8 open
  work, §9 tests 7-13.

Explicit DIFFERS vs core-budgetwindow4 (for the core reconciler):
- BW7: scroll step 2 while `pos + 9 < len` (not bound `len - 9`); down shows up unconditionally; no common
  refresh tail after buttons.
- BW8: group fraction is the f32 product `f32(pos) * 0.01f` (pos 30 → `0x3E999999`); levels and restore records f32.
- BW6a: panel Releases `*out` after the modal run; `+0x0C` released by on_shutdown or the destructor.
- BW6b MATCHES core (resume unguarded); the pause is a counter with no floor (failed create → −1).
- BW9: returns specified (slider/combo/ignored 0, close keys 1, handlers pass through); on_shutdown body specified.
- BW4: the commands write primary `+0x194` (not `+0x18C`) and call ShowWindow.

Open: BW5 (meaning of key modifier bit `0x4`; plain Enter byte-proven to do nothing) needs a game check. Not
covered: initIndividualBudget restore-record fill point; whether generator `+0x1C` AddRefs the proc; pixel layout.

Tooling: export via re_query.py and build-runs/*.asm dumps; REA not used this round.

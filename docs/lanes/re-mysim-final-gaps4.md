# re-mysim-final-gaps4 — answer MS13/MS14/MS15 (mysims §4.6/§4.7/§4.7.1/§4.12/§7.2)

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-mysim-final-gaps4 (answer MS13-MS15 mysims location/import/commute contracts) | `re-mysim-final-gaps4` / `.claude/worktrees/re-mysim-final-gaps4` | in progress | see Handoff |

## Handoff
- Goal: audit preserved WIP and resolve only public MS13–MS15 with byte/decompile evidence; docs only.
- Done: audited scope of 4c714151 (27 spec lines, not yet verified); targeted specs have no other delta against main. Existing build-runs/rd.py modification is a read-only PE constant reader, not evidence of native execution.
- Done this round: verified MS13 location bytes and corrected rounding/local-copy wording; proved MS14 zero-initialized family count, signed key formatting, short-name whole-stem and missing text skip branches. Native-derived examples are arithmetic/formatting checks, not game measurements.
- Milestone: dfc58173 verified location/import edge rules. Completed MS15 caller configuration, occupant-bound rectangles, scaled getter/low32 conversion and FailCommute-before-Release ordering; §§4.6/4.7/7.2 and five proposed §12 cases. Moved MS13–MS15 to answered with DIFFERS and subpart MATCHES; filed MS16 for unproved inherited factory configuration.
- Validation: questions.py index regenerated (mysims open 1 / answered 15 / next MS17), questions.py check PASS, git diff --check PASS. No engine/tools/ABI change; no game run or cargo/clippy/ctest required for this documentation lane.
- Next: final evidence/wording review, replace Handoff with report, commit all docs, write docs-kind ready queue at actual detached HEAD.
- MS15 trip proof: active = state nonzero (004277AB→00449550); subject uses new state at 00427980; force zeros local location world AND cell at 00427C60; congestion PositionToTract uses local world; ratio local cell. Detailed static asm files in build-runs/ms15-trip-asm.txt and ms14-import-asm.txt (do not commit exports).
- Facts: detached HEAD is intentional; do not update refs. q.py cannot start Bash (E_ACCESSDENIED), so use capped PowerShell/local Python. CheckCommute pseudocode has broken variable recovery; arguments/order proved with bytes. Factory 0070D860 lacks static decode/decomp, raw disassembler lacks capstone; REA returned target_unavailable (No app is open), so MS16 stays unproved. Inherited rd.py change audited, saved to ignored rd-audited-round2.py, restored; no helper code shipped.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-mysim-final-gaps4-r2-manifest.json; no engine/tools/ABI edits, runtime tests or subagents.


## Coordinator quota pause (2026-10-22)
Provider weekly limit; resets 18:00 Europe/London. This round stopped. Existing work is preserved as WIP, not ready or verified. Resume only after the provider limit clears and fresh slots/check --resume pass; the round still counts.

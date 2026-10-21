| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-tutorial4 (validate/finish cFC4TutorialSystem, enable = tutorial) | `glue-tutorial4` / (engine lane) | in progress | - |

## Handoff
- Goal: validate/finish existing cFC4TutorialSystem (glue15, `enable = tutorial`), find owner/contract gaps, meaningful in-game tutorial scenario, full gate.
- Done: TU6-TU9 filed 894c2bf5 (QUESTIONS.md not regenerated: tools/ off-limits); endTutorial 3D view slots 60/59 44b67160; script action `tutorial` 202af346.
- Game: glue-tutorial4-orig-1 (original class, tutorial_shadow, script build-runs/tut-script.txt, turbo to 2032) clean ref0, shadow 148/148; trace: end0 ret0, end1 ret1, event ret1, reset ret0, task 7->8.
- glue-tutorial4-ours-1 (enable = tutorial) clean ref0, script trace identical to orig (11 lines) but no shadow lines (shadow skipped ours); fixed a6a95a3b; glue-tutorial4-ours-2 clean ref0, trace identical, shadow 148/148.
- Next: qbuild all (final gate), then the report + merge-queue json.

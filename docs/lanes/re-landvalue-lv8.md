| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-landvalue-lv8: native capture lifecycle contract | Codex / `re-landvalue-lv8` | in progress | Verify LV8 dispatch exclusion and document implementable capture lifetime. |

## Handoff
- Goal: answer LV8 from glue-landvalue-step6; spec text only, no engine reads/edits.
- Done: initial handoff committed `64ea7840a`; verified native lifecycle, pause, dispatch and teardown instructions.
- Done: added §6.1.4 negative native-boundary answer, explicit host contract and vectors 26–31; archived LV8 verbatim from step6.
- Next: regenerate/check question index; run doccheck and review spec diff, then commit final report and queue.
- Main evidence: `C:/Users/tlori/FreeCity4`; worktree is detached by instruction, commit without changing refs.
- LV8 source: `.claude/worktrees/glue-landvalue-step6/docs/specs/questions/landvalue.md` in main.
- Preserve LV2/LV7 runtime verification gaps; no game or build requests outstanding.
- Source lane now has Report, no Handoff/Owner closure table; used its explicit LV8 and §6.1.2 acceptance obligations.
- Native pre-init/pre-shutdown provide ordering only, not a global admission/drain barrier. Host implementation still needs acceptance evidence.
- Global route lifetime can span cities; owner read lifetime must end before native Shutdown invalidates inputs.

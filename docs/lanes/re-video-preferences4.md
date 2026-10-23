| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-video-preferences4: native video preference panel contracts | Codex; re-video-preferences4 | in progress | Establish export identity; specify owner mappings and event chronology. |

## Handoff

- Goal: bounded first behavior specification for cSC4VideoPreferencesWinProc, no engine or foreign algorithm work.
- Done: first substantive mapping/traces committed as 0ea8f272. Complete SetPreferenceOptions normal CFG/29 operation rows now documented; message admission, EndDialog, Release and owner-attributed shared QI/Shutdown instruction-reviewed.
- Tool error: q.py Bash wrapper failed with Bash/Service/CreateInstance/E_ACCESSDENIED; use direct PowerShell commands with capped output.
- Identity: canonical export SHA-256 matches measured bin/SimCity4.exe; recorded in spec §1.
- Next: qualify SetButtonsBasedOnPreferences, then owner event close/default/save branches and initialization. Add full readback and event traces/questions.
- Git emitted packed-refs.lock permission error during commit, but committed detached HEAD 0ea8f272 successfully; do not alter shared refs/config.
- Authority: main build-runs/run4-re-video-preferences4-r1-manifest.json; no runtime/game/preferences writes authorized.

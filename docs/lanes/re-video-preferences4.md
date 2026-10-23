| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-video-preferences4: native video preference panel contracts | Codex; re-video-preferences4 | in progress | Establish export identity; specify owner mappings and event chronology. |

## Handoff

- Goal: bounded first behavior specification for cSC4VideoPreferencesWinProc, no engine or foreign algorithm work.
- Done: reused reserved detached worktree; read operating and lane publication rules. First substantive spec: four ordered byte assignments, lookup/release sequence and three independent literal traces.
- Tool error: q.py Bash wrapper failed with Bash/Service/CreateInstance/E_ACCESSDENIED; use direct PowerShell commands with capped output.
- Identity: canonical export SHA-256 matches measured bin/SimCity4.exe; recorded in spec §1.
- Next: complete remaining SetPreferenceOptions mappings, then SetButtonsBasedOnPreferences and HandleControlMessage CFGs.
- Authority: main build-runs/run4-re-video-preferences4-r1-manifest.json; no runtime/game/preferences writes authorized.

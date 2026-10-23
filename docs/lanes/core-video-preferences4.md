| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-video-preferences4 (bounded portable panel owner) | Codex / core-video-preferences4 | in progress | Implement published readback, qualified actions and lifetime; verify portable fixtures and full gate |

## Handoff
- Goal: independent Rust video-preferences owner, spec sections 2-7 only; no ABI/plugin/native integration.
- Base HEAD: 8e95854da8454d89131af5b7f1735f02c7b0cae2; publication commit 48a401f2 is present.
- Read rules, lane format, video-preferences sections 1-8, VP1-5 and question filing rules.
- Native VP-T1..24 remain PROPOSED / UNEXECUTED; portable fixtures will be independently transcribed.
- Next: write literal fixture data before production comparison, implement raw boundary and readback.
- q.py cannot start Bash (E_ACCESSDENIED); use bounded PowerShell output and approved qbuild wrapper.
- Task explicitly says no pre-edit baseline; run one canonical all after focused checks.
- No game requests; no native owner launch or complete redraw/default claims.

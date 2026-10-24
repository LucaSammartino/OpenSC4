| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-weather-boot8: timing-independent weather shadow bootstrap | Codex, `glue-weather-boot8` | in progress | Reproduce train183, fix native date stepping, validate to 2032 |

## Handoff
- Goal: weather shadow exact 73/73 through 2032 with scres_file_shadow on and off.
- Worktree reused, detached HEAD; initial tree clean.
- Baseline verify: build PASS 249/249, CTest PASS 78/78, logs in build-runs/fc4verify-{build,ctest}.log.
- Next: collect weather-boot8-before-on-02 verdict, inspect first difference, fix bootstrap/date stepping.
- Outstanding game request: weather-boot8-before-on-02 (house train183 switches, turbo to 2032).
- weather-boot8-before-on-01 rejected (submitted before DLL existed); no questions filed.
- Do not read or edit tools/; coordinator must handle any regress profile update.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lua-nativebinding4: LH1/LH2 native binding | Codex / `re-lua-nativebinding4` (RE docs, detached HEAD) | in progress | 1. Trace creation, service lifecycle and foreign command ABI. 2. Answer LH1/LH2 and publish binding limits. |

## Handoff
- Goal: answer only LH1/LH2; docs only, no engine/tests reads or changes.
- Worktree reused, clean detached HEAD; no branch/ref changes permitted.
- Done: substantive native identity/typed-boundary spec: CLSID CBD101A9 registration, service ID 4BD10133,
  priority -999999; CommandServer public acquisition EB903A32/CB90253A and generator sibling views.
- Next: finish framework Add/Init ordering, factory precedence and caller caching; verify generator retention,
  failure/shutdown, add independent discriminators, then move LH1/LH2 to answered and regenerate index.
- Key bytes: app factory-first request 00450813–0045082F, direct fallback 00450831–00450858,
  service QI/add/release 0045085B–00450891; SC4 director registration 0046021A–00460227.
- Export xrefs to factory missed its instruction reference; native PUSH in director proves registration.
- SDK: main external/gzcom-dll/gzcom-dll/include (not engine copy). Generator generic declaration exists.
- Ignored helper build-runs/native_tables.py reads PE vtable words; no external material copied into specs.
- Main RE inputs: `C:/Users/tlori/FreeCity4/{decomp,ghidra/export/windows,re}`; edits only here.
- Guard: `C:/Users/tlori/FreeCity4/build-runs/run4-re-lua-nativebinding4-r1-manifest.json`.
- Budget 150000, maxCalls100, maxRounds1; distinct native-binding phase, no restart/fourth logical phase.
- q.py Bash wrapper fails with E_ACCESSDENIED; use local PowerShell with capped output.
- No runtime runs, owner/default/coverage claims or ABI-table edits authorized.

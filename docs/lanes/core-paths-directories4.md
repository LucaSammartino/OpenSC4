| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-paths-directories4: portable path and directory entry owners | Codex / `core-paths-directories4` (engine lane) | in progress; detached HEAD reserved by supervisor | Audit existing primitives; implement published contracts and independent vectors; stable full gate |

## Handoff
- Goal: exact portable cRZPath/cRZDirectoryEntry and published path helpers from paths.md sections 1-4, 8.
- Reserved existing worktree reused; initially clean, no lane-only commits.
- Read paths sections 1-4/8, existing rz_files host/file primitives and public SDK filenames.
- q.py cannot launch Bash (E_ACCESSDENIED); use capped native PowerShell reads and qbuild for checks.
- Latest brief requests focused checks then one stable full gate, with no redundant baseline.
- No game runtime bindings, C ABI or plugin changes authorized; no game requests planned.
- Implemented byte path helpers and five-string RzPath, native find primitive/RAII handle, list-owned entry state and listing policies; independent tests added.
- Audit: no rz_services or path helper implementation exists here; reused rz_files OsHandle/OsError rather than duplicating primitive types.
- Owner checklist cRZPath: constructor/Drop (five owned Vecs), SetPath, four getters, lazy FileName, two setters, SetBuffer and three joined outputs implemented.
- Owner checklist cRZDirectoryEntry: published owner/name/kind state implemented as directory-bound borrow; standalone method contracts absent, complete-owner credit withheld (PA5).
- cRZDirectory listing supports §§3.1-3.4 only; recursive/create/remove are outside this owner task, not claimed.
- Questions PA4-PA8 appended to docs/specs/questions/paths.md; existing PA3 normalise conflict preserved, §§1.2/6.9 followed literally.
- Focused qbuild quick running in exec session 20449; no code edits until it completes.
- Next: inspect focused result, fix failures/format touched files, stable qbuild all, final report and queue.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-paths-directories4-r1-manifest.json.

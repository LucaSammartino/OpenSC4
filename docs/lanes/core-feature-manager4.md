| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-feature-manager4 (portable feature manager) | `Codex` / `core-feature-manager4` (engine lane; detached HEAD) | in progress | Audit public owner contract, implement portable core, validate and report. |

## Handoff
- Goal: implement the published cSC4FeatureManager portable core only; no C ABI/plugin edits.
- Done: public contract and open questions audited; implemented owner, explicit shared process state, resource scan, lifecycle, ordered entries, queries, virtual pack dispatch and 10 focused tests.
- Done: reused RZ byte-line parser; extracted existing Rule N into core and retained formats API via re-export. FM3/FМ4 appended (file read failures; AddRef overflow).
- Next: finish running quick check (session 27326); add independent public conformance vectors; format-check touched files; freeze source and run full gate.
- Facts: task-specific instruction skips pre-edit full baseline; run focused quick checks then one frozen-source full gate.
- No game requests; core-only lane. Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-feature-manager4-r1-manifest.json.
- Shell fact: q.py cannot start Bash (E_ACCESSDENIED); native PowerShell works. qbuild.py quick is running.

## Owner checklist (published bodies only)

| cSC4FeatureManager body | Status | Portable implementation |
|---|---|---|
| constructor | complete | FeatureManager::new; zero count, uninitialized |
| QueryInterface slot 0 | complete | same token for both IIDs; AddRef; failure preserves output |
| AddRef slot 1 (shared body, outside 12 unique owner bodies) | complete; FM4 overflow choice | 32-bit count increment/register bits |
| Release slot 2 | complete | signed >1 decrement; otherwise Shutdown and destructor flag 1 |
| Init slot 3 | complete | object guard; static scan; always true while live |
| Shutdown slot 4 | complete | object flag only; statics retained |
| IsFeatureEnabled slot 5 | complete | unsigned ordered-set lookup |
| IsExpansionPackInstalled slot 6 | complete | low-five-bit wrapped shift |
| GetCurrentExpansionPack slot 7 | complete | descending virtual-query default on same object |
| deleting destructor slot 8 | complete | returned identity; bit 0 frees; no implicit Shutdown |
| static scan | complete for readable raw files; FM3 streaming failure missing | guard first, register, ascending 32 raw keys, bit before parse, release each opened file |
| Features entry callback | complete | shared Rule N; exact case-insensitive off; ordered insert/erase |
| static shutdown callback | complete | guarded static reset; no unregister |

Compiler-generated set startup/teardown are explicitly exempt in §2. App creation/getter/caching and native heap/vtable bindings belong to integration, not additional owner methods. No C ABI, plugin, ABI-map or game changes.

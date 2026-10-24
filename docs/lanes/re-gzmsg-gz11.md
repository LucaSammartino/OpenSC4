| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzmsg-gz11 (GZ11/GZ12 observation and acquisition) | Codex / `re-gzmsg-gz11` | in progress | Verify native observation and existing-manager acquisition; document vectors and run doccheck. |

## Handoff
- Goal: answer GZ11/GZ12 in framework-gz-services; spec text only.
- Done: drafted §§3.1/3.2 and eight vectors GZ-V15–V22; moved GZ11/GZ12 to answered archive.
- Findings: class 0x452294DF is registered as factory only; normal application service setup omits manager. Existing lookup pair is correct; absent service leaves output untouched.
- Findings: manager slot 9 (0x009DB663) returns unsigned ID-entry count without locking; no complete snapshot/enumerator/hook on recovered interfaces.
- Next: regenerate question index, review evidence/diff, run doccheck and questions check, then final report/queue.
- Native failure input: service 0x64FA8434, IID 0x652294BC; do not instantiate or insert native services for diagnostics.
- Worktree HEAD is intentionally detached; commit normally without changing refs.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-gzmsg-gz11-r1-manifest.json.

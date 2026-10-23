| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-draw-context-dc9 (TurnOnFog contract) | `Codex` / `re-draw-context-dc9` (RE lane) | in progress | Trace fog inputs, cache and failures; add spec vectors; doccheck. |

## Handoff

- Goal: answer DC9 with signature, sources, cache order, driver calls and retries.
- Done: §1.5.1 fog contract and nine spec conformance vectors DC-V27–DC-V35; DC9 moved to archive.
- Validation: questions index regenerated; questions check and doccheck check passed. Removed trailing blank flagged by diff check.
- Next: commit milestone, then final report and queue.
- Facts: native void thiscall, 16 argument bytes; enable/RGB/start/end caches update after calls; failures ignored.
- Evidence: 0x007D32D0 instruction pages and four renderer callers; constant 0x00ABB4D0=0x7F7FFFFF.
- Decompiler's final pointer store is wrong; bytes at 0x007D33E2–0x007D33E6 cache end argument.
- Do not read/edit engine; optional owner-table inspection conflicts with explicit prohibition and is skipped.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-draw-context-dc9-r1-manifest.json.

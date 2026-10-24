| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-effects-ef20 (native capture contracts) | Codex / `re-effects-ef20` | in progress | Verify EF20 collection and capture ABI; document behavior and conformance cases |

## Handoff

- Goal: close EF20 from glue-effects-shadow7 with spec-only native capture contracts.
- Done: initial handoff committed as `89a039de0`; EF20 answered in effects.md §18.7 with ten proposed spec vectors and archived source question.
- Source lane file now contains a final report; no Owner closure table or `open:` rows remain there. EF20 is the remaining scope restriction in its landing report.
- Next: regenerate questions index, run doccheck, review diff and commit final report/queue.
- Verified: bool is AL-only; no collection enumeration slots; getters mutate scratch; resource slot 5 swaps storage for an empty destination; queue records stride 0x2C.
- No engine, plugin, capi or ABI table edits; no game requests outstanding.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzframework-gzf5: GZF10/GZF11 contracts | Codex / re-gzframework-gzf5 | in progress | Recover original contracts, extend spec vectors, answer questions, doccheck. |

## Handoff

- Goal: answer GZF10/GZF11 in gz-framework-core with behavior and conformance vectors.
- Done: GZF10 answered in §§2, 5–8; added GZC39–52; corrected span-render-8 Unlock wording; archived GZF10.
- Next: finish GZF11 stream/COM/command/log contracts and vectors; archive it, regenerate index, doccheck.
- Facts: ordered priority descending signed, stable ties; Unlock masks supplied bits; logger owner captured by Init; failed path setter preserves validity but replaces path.
- Facts: buffer Release at zero underflows; support final Release deletes without Shutdown; framework slot 22 returns a byte flag.
- Engine paths are forbidden; owner closure gaps read from docs/lanes/core-gzframework5.md §§Remaining gaps, not its linked engine table.
- No REA use: export/decomp only as requested. No game runs or ABI changes planned.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-gzframework-gzf5-r1-manifest.json.

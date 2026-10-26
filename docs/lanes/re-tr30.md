| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-tr30 terrain-view replacement questions; kind docs | Codex / .claude/worktrees/re-tr30 / re-tr30 (detached HEAD) | ready; spec commits c9de4007d, 964149191; doccheck passed; ABI unchanged; game n/a; shadow n/a; TR45 answered, TR46 narrowed | 1. Obtain the native captures in terrain.md §15.5.15.4 before claiming complete frame parity. 2. Cover the native lifecycle boundary in §15.5.14 before independent view registration. |

- Imported only docs/specs/questions/terrain.md from replace-terrainview29; retained the other questions from main.
- TR45: no interface-only lifecycle route; specified base/interface identities, direct Init/Shutdown/index calls, externally read fields, serialization and fresh/load worked cases. Corrected view3d.md §14.
- TR46: specified all split/winding/hide index combinations, row-relative indices, Update/land/water/cliff ordering, plain row arguments and alpha mutation timing, with worked submissions. Corrected terrain-view.md's index origin.
- TR46 remains a narrowing: full cache bytes, frustum rounding boundaries, auxiliary passes and extension batching require the explicitly listed native observations. No complete-frame parity or replacement credit is asserted.
- Both entries and provenance moved to docs/specs/questions/answered/terrain.md; regenerated QUESTIONS.md.
- Gate: final direct python tools/doccheck.py check passed; whitespace check passed; no engine/code/test edits or ABI row proposals.
- Global questions.py check reports nine existing issues outside terrain (four unparsed IDs and five open/answered duplicates); no terrain issue reported. Those files were not changed.
- A q.py-wrapped final doccheck attempt failed before execution because Git Bash CreateFileMapping was denied; the later direct Python doccheck passed. Kept distinct from the passing gate.
- Git printed packed-refs.lock permission errors during commits, but both commits succeeded on detached HEAD; no shared-ref repair attempted.
- Sources: read-only main-checkout export queries and decompilation; numeric/index claims checked against instruction bytes. REA not used. No game run or shadow comparison.
- Next: capture ordered native context calls and draw-time buffer bytes on the fixtures and capability modes specified in §15.5.15.4.

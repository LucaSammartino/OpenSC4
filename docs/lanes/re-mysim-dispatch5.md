| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-mysim-dispatch5 (dispatch trip contract, MS17) | Codex / `re-mysim-dispatch5` | in progress | Trace dispatch transitions, record bytes and RNG; extend spec and disposition MS17 |

## Handoff
- Goal: specify MS17 dispatch owner without engine edits.
- Done: initial handoff committed f9379fb6. Added dispatch state/queue/time contract, serialized record, request/player entry points, tuning and eight spec vectors; moved answered MS17 parts and filed residual MS18.
- Next: validate spec cross-references, question index, numeric citations and final diff; commit report and queue ready.
- Evidence: main checkout `C:/Users/tlori/FreeCity4`; export query `tools/re_query.py`.
- No game requests, ABI proposals or engine checks planned (documentation-only RE lane).
- Important: dispatch serializer receives object+0x0C; clock messages add 5; wait getter returns switch-start timestamp and clears it after >15 only off-switch.
- Partial scope: complete route-ready multimodal handoffs and delegated automata RNG remain MS18; no immediate-arrival stub permission remains.

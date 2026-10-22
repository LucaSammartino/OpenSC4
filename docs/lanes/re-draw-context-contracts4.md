| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-draw-context-contracts4 (DC3–DC8 native contracts) | `re-draw-context-contracts4` / (RE spec lane, detached worktree) | in progress | (1) Reconcile native owner inventory; (2) resolve DC4–DC8 from instructions and data; (3) publish residual gaps. |

## Handoff
- Goal: exact native contracts for DC3–DC8 in draw-context.md; spec text only.
- Done: first native proof in §1.2–1.3: all combiner bytes; white shade; lifecycle helpers; callback receiver disproves Begin/End inference.
- Done: first milestone a0a0d989. Added native inventory, foreign driver targets, full stage defaults and operation-specific gate/order/cache/failure/retry rules.
- Done: a81f458d stage/order milestone; added §§8.1–8.3 shade, binary32/binary64/x87 rules, full material texture matrix and 14 independent instruction-derived vectors.
- Questions: DC4–DC7 answered DIFFERS and moved; DC3/DC8 retain exact residual contracts. DC1/DC2 untouched.
- Next: index/check questions and whitespace; finalize report/queue as wip (stack mutation and mixed-transform spill/grouping still unresolved).
- Key correction: 0x007D30D0/0x007D32D0 use context receivers despite renderer export labels; TurnOnLighting populates default shade via lighting slot 10.
- Stack: top replay reads end-8; no push/pop body established. 0x007D4F80 writes highlight colours; 0x007D4B80 is a raw-matrix transform path.
- Measured: 12-byte records include explicit word-zero tails; stage coordinate=0, wrap/filter=-1; SetTexture has no selection/equality/refcount calls.
- Main evidence: C:/Users/tlori/FreeCity4/ghidra/export/windows and decomp/SC4DrawContext.c; query with main tools/re_query.py first.
- No engine reads/edits, ABI edits, runtime claims, subagents, network, branch changes, or main-checkout edits.
- q.py Bash wrapper cannot start (E_ACCESSDENIED); use native PowerShell with bounded output.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-draw-context-contracts4-r1-manifest.json.
- Open questions: DC1/DC2 pre-existing; DC3–DC8 this lane. No game requests.

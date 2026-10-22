| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-draw-context-contracts4 (DC3–DC8 native contracts) | `re-draw-context-contracts4` / (RE spec lane, detached worktree) | in progress | (1) Reconcile native owner inventory; (2) resolve DC4–DC8 from instructions and data; (3) publish residual gaps. |

## Handoff
- Goal: exact native contracts for DC3–DC8 in draw-context.md; spec text only.
- Done: first native proof in §1.2–1.3: all combiner bytes; white shade; lifecycle helpers; callback receiver disproves Begin/End inference.
- Next: finish attributed owner inventory/stack and stage cache/order/failure contracts; numeric/shade/transform vectors.
- Measured: 12-byte records include explicit word-zero tails; stage coordinate=0, wrap/filter=-1; SetTexture has no selection/equality/refcount calls.
- Main evidence: C:/Users/tlori/FreeCity4/ghidra/export/windows and decomp/SC4DrawContext.c; query with main tools/re_query.py first.
- No engine reads/edits, ABI edits, runtime claims, subagents, network, branch changes, or main-checkout edits.
- q.py Bash wrapper cannot start (E_ACCESSDENIED); use native PowerShell with bounded output.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-draw-context-contracts4-r1-manifest.json.
- Open questions: DC1/DC2 pre-existing; DC3–DC8 this lane. No game requests.

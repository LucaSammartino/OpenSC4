| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gdrivers-director4: graphics COM director native behavior specification | `re-gdrivers-director4` / (RE lane, detached worktree) | ready for docs train; final round 3/3; 65/65 attributed bodies, 8068/8068 bytes have contracts; 65 independent paper vectors; native checklist/dispatch audits PASS; cargo/clippy/ctest n/a; game n/a; ABI unchanged; reconciliation 1433967f | 1. Coordinator publish detached commits and consume updated queue. 2. PARK outside-inventory dependencies and capped driver/RD3–RD5; no fourth round or whole-family credit. |

## Final report (2026-10-22, round 3/3)

The final specification consists of [the director contract](../specs/renderer-gdrivers-director.md)
and [the attributed-body contract and checklist](../specs/renderer-gdrivers-director-inventory.md).
The latter §7 identifies every attributed address, exported size, attribution rule and contract
section; §8 records the final proof reconciliation and boundaries. This report replaces the stale
round-1 85-byte completion/discrepancy report. The existing merge-queue record is replaced after
this report is committed, using the actual detached HEAD.

### Changes and milestones

- `8fc7231c`: inspected and committed the two preserved pending corrections first. The aggregate
  default template initializes its key, nine entry pointers, only the first eight associated value
  words, and its flag; the ninth value is unspecified. Parent attribution wording now includes
  reach/neighbor rules rather than calling every body reachable.
- `1433967f`: reconciled the full inventory, corrected the first constructor factory-argument
  citation to `0x00466258` and distinguished registration calls `0x00466272`/`0x00466284`.
  Preserved the exact product-name bytes `Maxis 3D GDriver\nOpenGL\n1.1\n`, including the final
  line feed, from pointer `0x00B1225C` to string `0x00AC3F44`. Added final proof/boundary §8 and
  removed stale parent wording about the additional contracts still being required.
- Prior substantive contracts remain in `b365703d` (factory/constructor/allocation failures,
  pool and coefficients), `4cf51c6f` (OpenGL dispatch, capture, ownership and metadata), and
  `ff6cf97d` (software buffers, save/restore/capture, aggregate storage and cleanup). Round-1
  director dispatch/lifecycle contracts and GD1–GD31 remain intact. No 65-body re-investigation
  or capped-driver tracing was repeated this round.

### Measured inventory and behavior

A fresh read-only attribution from the main checkout's native export/executable matches the
round-2 address/size/rule snapshot exactly: **65 bodies / 8,068 bytes**. The native rule subtotals
are name-medium 1/79, name-low 1/6, reach 56/7,088, free-neighbour 1/6, neighbour 6/889. Thus
**85 directly named bytes + 7,983 reach/neighbor bytes = 8,068**. No row, neighbor attribution or
failure path was excluded, and no ownership/map changes were proposed. Every exact-set row has
its native-derived behavior contract. This is static specification coverage, not executed proof.

The parent contract additionally specifies all **17 primary slots** and **11 secondary entries**.
The 16 shared primary targets total 506 bytes; these and other supporting functions are separate
dispatch evidence, not bytes to add to the 8,068 denominator. Registration, initialization/service
binding, factory ownership/failure, class enumeration, references/unload, configuration, no-op
lifecycle callbacks and cleanup follow the actual director inventory. The specification does not
invent renderer start/stop or device-enumeration methods.

**GD1–GD65** are independent discriminating paper vectors for lifecycle, registration/order,
allocation and interface failures, capture, resource reuse, rectangle operations and destruction.
They are requirements derived from primary native control flow, not implementation or game results.

### Evidence and validation

Primary sources were the main checkout's `tools/re_query.py`, `ghidra/export/windows`, `decomp/`
and local `bin/SimCity4.exe`. Export queries preceded relevant native reads. The final default
template correction was checked against `0x008DF260–0x008DF2DE` instructions; factory argument
and registration citations against `0x00466250–0x0046629E`; string punctuation against native
PE bytes and the constructor's pointer load/length walk at `0x0087E003` onward. No REA was needed.
Bounded PowerShell calls were used because the prior q.py Bash launch failed with E_ACCESSDENIED.

Ignored worktree artifacts: `build-runs/director-attributed.tsv`, the preserved round-2 snapshot,
and `build-runs/director-attribution-round3.out`. `audit-director-round2.py` passes all 65
addresses/sizes/rules and contiguous GD1–GD65; `audit-director-doc.py` passes the direct inventory,
17/11 dispatch entries, shared target sizes and enumeration residues. These audits check document
consistency with native metadata/PE data; their output is not behavior evidence. `git diff --check`
passes. Cargo, clippy and ctest: **n/a**, documentation only. Game runs, shadow exactness and
replacement claims: **n/a**. No game requests are outstanding.

### Questions, boundaries and disposition

Questions filed/answered: **none**; no question-index change was needed. ABI exports, ABI tables,
ABI proposals, engine, tools, maps, game files and external files were not changed. The worktree
remains detached and preserved; no shared-ref update, merge, push, restart or subagent was used.

No residual row inside the exact 65-body inventory lacks a contract. **PARKED outside this claim:**
the third factory's capped DX7 product, RD3–RD5, delegated OpenGL mode/device helpers, other
named/shared owners and general allocator/string/container APIs beyond the specified calls.
There is no transitive owner or whole-DX7-family completion credit. This final round is ready for
the docs train; the coordinator publishes the detached commits. No fourth round is requested.

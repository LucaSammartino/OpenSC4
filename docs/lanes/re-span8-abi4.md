| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span8-abi4 (SR9 native ABI evidence) | `re-span8-abi4` / RE, detached worktree | ready; SR9 native contracts resolved; 40 absent-key proposals; 1,782 instructions / 43 ranges exact; questions/whitespace PASS; cargo/clippy/ctest and game n/a; no C ABI change | (1) Coordinator reviews/applies absent keys through gated train; (2) glue implements SR10 adapter before bridge enablement. No fourth RE-family round. |

## Report (2026-10-22)

SR9's native call contracts are resolved in [span-render-8.md §§10.1, 10.5, 10.7](../specs/span-render-8.md).
The six cSpanRender8 slots are confirmed from the actual vtable dwords and instructions. Slot 4 inherits
16-byte cleanup by tail-jumping to slot 2. Context slot 22 is void, pops 8, has eight native return sites,
and its triangle/quad/polygon rasterizer callers ignore EAX. This strengthens the older proposal's unknown
return while preserving the stronger existing public spec. Public setter semantics remain the shared proof
in span-render-565.md §1.3/§6.2; buffer descriptors include returned output pointers, rather than guessed voids.

Commits: `ff5283b8` is the early native-boundary milestone; `84b01b5c` publishes the full evidence, SR9 answer
and question reconciliation. This final report replaces the working handoff. Work was committed on the
reserved detached HEAD; the supervisor publishes it to the reserved branch.

## Proposal for coordinator review

Artifact: `C:/Users/tlori/FreeCity4/.claude/worktrees/re-span8-abi4/build-runs/abi-rows-re-span8-abi4.tsv`.
It uses the existing 12-column interfaces.tsv schema and contains **40 unique absent keys**:

- 6 cISpanRender / cSpanRender8 / 00ac188c rows, slots 0..5.
- 13 cIGZDrawContext / cGZDrawContextRenderToBuffer / 00ac17d0 rows, slots 0/1/2/3/4/7/8/9/12/13/14/15/22.
- 1 cIGZDrawSystem / cGZDrawSystem / 00ac1668 row, slot 4.
- 20 cIGZBuffer / cGZBuffer / 00ac1400 rows, slots 0/1/2/3/6/7/9/10/11/15/17/26/30/34/35/40/42/43/44/45.

Buffer lifecycle closure covers Init's allocation calls and final Release's deleting destructor/free calls;
slot 45's type-2 path leaves the descriptor unchanged. No palette-service, alternate buffer, factory or
antialiased direct-pixel methods are proposed. All conventions are thiscall; no pop count or return is unknown.

SHA256: `0641b79e9fc283434af58419724c4306a98361e4ce4f943d0e8185f506107165`.
The tracked spec tables retain every proposed target, argument list, cleanup and return contract for review;
the TSV and scratch audit remain local build-runs artifacts as requested.

Exact-key reconciliation against the main checkout's interfaces.tsv (observed main `47efb91e`) found **0 present,
40 absent**. The earlier 17-row proposal overlaps 14: **13 matching, 1 unknown-to-void strengthened** (slot 22).
Its other three context rows, Suspend/Resume/LockTarget, are outside this route and untouched. Review the exact
interface/class/vtable/slot key again when applying; preserve any stronger intervening rows. This proposal is
not applied native ABI approval. No interfaces.tsv, unreconciled-rows.tsv, ABI version, engine or tools edits;
no C exports changed, and no new native forwarders or fixture calls were introduced.

## Validation and limits

Native image SHA256: `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
The scratch audit (`build-runs/audit-re-span8-abi4.py`, output `build-runs/native-re-span8-abi4.txt`) verifies
native vtable targets, unique proposal keys, prior-proposal overlap, dword/pop consistency, **all native RET
pops**, slot-4 tail cleanup, and **1,782 matching instruction byte sequences in 43 function ranges**.
These are static checks; no game, shadow, emulator fixture or compiled gate was run. Cargo/clippy/ctest: n/a.
Questions index/check and cumulative `git diff --check`: PASS. Only the lane report and span8 spec/question/index
files changed in Git.

The Bash q.py wrapper could not launch (CreateInstance E_ACCESSDENIED); capped PowerShell was used.
The export lacks the software factory entry 0x0082B910, and its disassembly fallback lacks capstone. The existing
factory proof is retained without claiming a new byte audit or proposing its ABI. All required new rows are
covered by decoded export instructions independently matched to native bytes. No REA calls or session errors.
Git printed a shared packed-refs.lock permission error during successful detached commits; HEAD advanced and
no shared refs/config were changed.

## Questions and next steps

SR9 moved to `docs/specs/questions/answered/span-render-8.md`, marked ANSWERED/DIFFERS for the older slot-22
unknown. SR10's already-answered coordinator glue-scope clarification is preserved in that archive, and its
duplicate open entry removed; §9 records the scope. Three open questions remain unchanged: SR8-2 reachability,
SR8-3 row13 texel reads, SR8-4 antialiased context output. No unresolved native contract remains in the bounded
SR9 route, and no new RE question was filed.

Coordinator review/application through the gated train remains necessary before enabling or exercising the
bridge. SR10's new checked C adapter is glue scope. No runtime/default/shadow/replacement or whole-owner
coverage credit is claimed. This is the final third span8 RE-family round; no fourth round or restart requested.

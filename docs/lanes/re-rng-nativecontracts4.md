| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-rng-nativecontracts4: RN4/RN5 native RNG contracts | `re-rng-nativecontracts4` / detached reserved worktree (RE spec lane) | Blocked: lane guard PARKED by coordinator STOP RE; RN4/RN5 remain open. Partial native findings preserved; no spec or ABI changes; tests/game n/a. | 1. Coordinator release required before further RE. 2. Complete storage, disposal and caller/insertion proof before publishing contracts. |

## Handoff
- Goal: answer only RN4/RN5 in foundations; specs only, no engine or ABI-table edits.
- Done: reserved detached worktree reused; clean initial status; RN4/RN5, foundations 6.1-6.4 and glue-rng4 read.
- STOP: next evidence batch denied by lane guard PARKED: coordinator directs STOP RE, prioritize verified/replaced. No more RE, index or merge-queue writes authorized.
- Next after explicit release: finish storage extent/alignment, destruction/lifetime, all methods and native caller/install/remove proof. Do not treat the following partial observations as a complete callable contract.
- Native constructor cRZRandom 0x00913A5C: ECX destination, one stack seed u32, calls Seed 0x00913667, clears dword +8, returns destination EAX, RET 4. No allocation in this body. Full extent/cache location not yet proved.
- Native constructor cRZRandomSuper 0x00913B74: ECX destination, one stack seed u32, clears dword +0x9C0, stores 624 at +0x9C4, calls Seed 0x00913A72, returns destination EAX, RET 4. Full extent/alignment/lifetime not yet proved.
- Export caller lists: 74 direct Random constructor calls, 17 Super constructor calls. Bounded samples include stack/member/static candidates; no insertion route proved.
- Random Seed 0x00913667 writes +0/+4 only, leaving +8/cache untouched; sentinel calls 0x009EFB59 then clock 0x009F361F; computes time XOR ((clock << 16) OR clock). Super sentinel reads clock before time, calls 0x00913977; helper not inspected.
- Random raw 0x00913696 uses unsigned MUL/ADD/ADC; EAX low32 of shifted 64-bit result, EDX also contains residual upper bits (do not infer a public 64-bit return from decomp).
- Random bounded 0x009136B1: ECX object, one u32 stack argument, EAX result, RET 4; n=0 returns zero without raw call. Super bounded 0x00913A06 always calls raw, including n=0, returns high multiply word EAX, RET 4.
- Signed uniform Random 0x009136CA / Super 0x00913A14: two stack i32 arguments lo/hi, modular subtraction/addition, EAX result, RET 8.
- Double uniform Random 0x0091372E / Super 0x00913A29 returns x87 ST0; signed FILD; Random checks +4==0 and calls clock. Range 0x00913761 / 0x00913A46 takes stack f64 a,b, ST0 result, RET 16.
- Random fast double range body is 0x0091386B (not the misleading 0x005CFE20 label); three calls to 0x0091372E, double spills for first two, adds third+second then first, scales (b-a), then constant at 0x00ACECB0, adds a; RET 16. Constant bytes not yet independently read.
- Decomp files exist for both owners; exported labels 0x0049FC70 and 0x005CFE20 are unreliable (func resolves other owners). Unnamed 0x009136DF size79 not yet inspected. Gaussian/cache, weighted method and Reload instruction proof remain incomplete.
- No independent native sequences captured; no fixture acquired, no native calls executed, no insertion installed. RN4/RN5 remain in open questions; no answered entries or ABI proposals.
- Native sources: C:/Users/tlori/FreeCity4/tools/re_query.py and decomp/ in main checkout.
- q.py cannot start Bash (E_ACCESSDENIED); use bounded native PowerShell reads/commands.
- No REA call yet; available only for export/decomp gaps.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-rng-nativecontracts4-r1-manifest.json.
- Do not switch detached HEAD; supervisor publishes commits. No subagents, engine reads, maps/tools/ABI edits.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gdrivers-director4: cSC4GDriversCOMDirector8068B native behavior specification | `re-gdrivers-director4` / (RE lane, detached worktree) | in progress | 1. Inventory attributed director bodies. 2. Specify lifecycle, registration, factories and failures with independent vectors. |

## Handoff
- Goal: specify only previously unspecified cSC4GDriversCOMDirector8068B; no DX7 driver-family credit.
- Worktree reserved and reused; detached HEAD; no branch/ref changes.
- Read re-driverdraw4 scope: renderer-driver.md covers capped nSGLDX7 driver; RD3-RD5 are out of scope.
- Done: scope commit 88bcbb84; renderer-gdrivers-director.md §§1–5 specifies constructor/ID, singleton, registration order, service binding, lookup ownership/failures and class enumeration; 12 paper vectors.
- Done: completed all 17 primary slots and 11 secondary entries, shared helpers, unload/reference quirks, exit destruction and native enumeration order; 31 discriminating paper vectors.
- Next: final native-evidence consistency review, documentation-only validation and final lane report/queue.
- Inventory: exact export owner has two named bodies (79+6=85 bytes), plus folded shared destructor (30 bytes); task suffix 8068B does not match direct attribution.
- IDs: director 0x64B2A903; classes registered in order C4554841,7ACA35C6,BADB6906 (kind 1).
- Important: kind-1 GetClassObject has no factory-null guard; failed QI calls AddRef then Release. InitializeCOM children receive original service/path (asm corrects pseudo).
- Order: 193 buckets; class residues 40,113,179 => enum 7ACA35C6,C4554841,BADB6906. Shared primary targets: 16 unique, 506 bytes; no exclusive/shared double count.
- Service path is a source string interface; copy length before data. Secondary slots 3–10 are true-returning no-ops, not inherited PreFrameWorkInit.
- Evidence: main checkout absolute paths; export/decomp/native instructions only. No engine reads or edits.
- q.py cannot start Bash (E_ACCESSDENIED); use bounded native PowerShell calls instead.
- No game requests, ABI proposals, or questions yet.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-gdrivers-director4-r1-manifest.json.

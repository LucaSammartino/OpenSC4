| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-command-system-controls4: bounded native-owner OFF/ON controls, original final3/3 | Codex / `verify-command-system-controls4` (detached HEAD) | in progress | Verify provenance and idle queue; run OFF then ON on frozen train108 pair. |

## Handoff

- Goal: exactly two new supported-profile controls; no source edits or rebuilds.
- Initial HEAD 640ba764c40a1fd2a959b6c396212fc75bd5736b; engine tree fcf40905fd2ea714a9f3dfa8bd8fd9da453b9af2 matches copied gate manifest.
- Copied evidence lives only in build-runs/evidence-run4; DLL SHA256 45de4459b606931d384a166726a0b1df8870465af8f84fff89cb58cf781fcd36 (18282496 bytes); PDB SHA256 3bf57b3ab7e88eda24b5c43415cb0071280f8bf86356108f3ebe5a9cdd8464ad (171528192 bytes), measured locally.
- Gate source bfb7fc14d7b70eaeba82ea9f1ee2088d9e29f761; copied gate/review says Rust5329/0, clippy clean, native190/190, CTest68/68. Historical gate evidence, not a new gate.
- PDB manifest at02:21:48 supplies source/before/after/destination proof, superseding the earlier DLL manifest's no-PDB claim.
- Public cmdline final report supplies enable empty / disable budget; only simulator effectively enabled; turbo1,420/600,end2032,save2031-12-01,mark2031-12-15,normal quit.
- Initial handoff committed 3bb72280; local provenance and idle checks passed at02:27:49 BST before OFF.
- OFF commands-controls4-r3-off-train108-20261023-a clean2032-01-07,44.141s real,56s wall; save ok1/11000ms,mark complete,all shutdown hooks through PostSystemServiceShutdown,ref0/foreign0/stall0/RTC0/overrun0. No outstanding request. Raw verdict retained unchanged; docs/runs same label records result.
- Lane-only read-only validator/waiter build-runs/commands-controls-check.py; same save/mark script build-runs/commands-controls-save.txt. Actual immutable request points directly to evidence-run4/FreeCity4.dll and adjacent PDB.
- OFF recorded4b0ccf47. Pair hashes/provenance/idle checked again02:30:33 before ON.
- ON commands-controls4-r3-on-train108-20261023-b clean2032-01-19,41.718s real,51s wall; save ok1/11031ms,mark/all2actions complete,full shutdown,all guard counts0/ref0. Actual7414checked/exact,0mismatch/refused,sets3708/parser3706,historical171.
- Audit02:32:45 confirms requests identical except label/verifier flag,both simulator only,watchdog5000/15000/10000,all lifecycle hooks. Native service game+6D17A0 acquired1/released1;factory3+parsed11 game+6D16C8 acquired14/released14,net0. Pair hashes unchanged after ON,no outstanding requests.
- Source checks: engine diff vs initial640ba764 exit0; command core/CAPI/header diff vs production82376d6b exit0. Next: replace handoff with final bounded report,record method denominator derivation,docs diffcheck,commit and ready queue. No further game requests.
- Historical unavailable DLL17115a0c... is separate from train108; old R2 exact7414 and171 logical-count differences, current40/reload and disable-all stalls remain historical only. GC1-GC7 excluded; GC8 settled.
- No engine/ABI/spec/default/tool/policy changes, no other-worktree reads, no external actions or subagents.

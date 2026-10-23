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
- No outstanding request found in this worktree; no requests submitted yet.
- Next: create lane-only provenance/queue validator and blocking waiter; hash pair before OFF, submit unique OFF request, accept existing verdict if already completed; stop on failure.
- Historical unavailable DLL17115a0c... is separate from train108; old R2 exact7414 and171 logical-count differences, current40/reload and disable-all stalls remain historical only. GC1-GC7 excluded; GC8 settled.
- No engine/ABI/spec/default/tool/policy changes, no other-worktree reads, no external actions or subagents.

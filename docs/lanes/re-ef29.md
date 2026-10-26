| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-ef29 / EF25 effects resource; kind docs | Codex / .claude/worktrees/re-ef29 / reserved branch re-ef29 (detached HEAD) | wip: EF25 answered; commits 245e8fbab (requested merge), e1b61ba67, f564a1457; scoped docs gate passed, global questions check failed on unchanged NT36; cargo/clippy/ctest and game/shadow n/a; C ABI exports unchanged | 1. Coordinator resolves baseline NT36 open/answered conflict. 2. Rerun questions check and promote docs queue entry to ready. |

- effects.md §18.9 supplies all nine primary slots, argument/result widths, thiscall cleanup, name ownership, and the primary-table boundary.
- QI identities: R, collection R+4, persist and canonical unknown R+11C; no serializable IID. Shared atomic count, zero-count construction and final destruction are specified.
- Factory and serializer bridge contracts include type/key checks, exact stream IIDs, reference events and receiver adjustment.
- Writer always emits root 4 and all native directory sections; normal Boolean success does not aggregate stream failures. Reader accepts roots 3..4 and mutates incrementally without rollback.
- Corrected §3 map terminator: exact `(end,FFFFFFFF)` pair, not a standalone length marker.
- Added four worked ABI/ownership cases and twelve proposed writer/reader vectors; none executed against the game. No replacement or whole-owner coverage claimed.
- EF25 moved to docs/specs/questions/answered/effects.md with the required 2026-10-26 answer marker; index regenerated; find EF25 resolves to the archive.
- Scoped check: `python build-runs/check-ef25.py` passed; generated index current, EF25 archived once/not open, effects IDs valid, diff clean.
- Global `python tools/questions.py check` fails only on NT36 at questions/network-tools.md:120, open and already answered. Both involved files are unchanged from HEAD; left outside this question's scope.
- Bash q.py wrapper failed before checks (Windows signal-pipe error); authoritative checks ran directly in PowerShell. Git emitted a packed-refs.lock permission warning but both content commits succeeded.
- Evidence: original export queries, decompilation ranges and instruction/byte checks. REA was not used; no engine files were read or edited. docs/abi was read only.
- Proposed primary-vtable rows: build-runs/abi-rows-re-ef29.tsv (nine rows, docs/abi/vtables.tsv format; ignored, not committed).
- Next: resolve baseline NT36 and pass the global question check; then ready the kind-docs queue entry. Engine lane replace-effectsres28 can use the completed EF25 contract now.

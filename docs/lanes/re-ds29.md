| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-ds29 — MapView snapshot/interface binding; kind docs | Codex / .claude/worktrees/re-ds29 / re-ds29 (detached HEAD) | ready; commits e81a61aa4 (requested drawshadow29 merge), 1f2e1e699, fef2c0bd4; ABI exports unchanged, no seam reservation; docs checks 3/3; cargo/clippy/ctest n/a; game/shadow n/a; DS29-1 answered | 1. drawshadow29 binds the owner snapshot and minimap surface events. 2. Run the first same-invocation comparison; no method credit claimed here. |

- Added map-view.md §1.3: 0x9E8 allocation, complete owner fields, interface adjustments, retained collaborators, highlight/string/vector/list records and comparison rules.
- Update takes the complete owner; notification DoMessage takes owner+4; UI DoMessage takes owner+8; the factory returns owner+0xE0.
- Loaded selection is an exemplar instance. Pending requests insert at the front, retain duplicates and remove only the first match.
- Added §4.4: cIGZBuffer creation/format, locks, bits/pitch, writes, references and the cSC4WinMiniMap IID 0xCA318385 SetOverlay boundary.
- SetOverlay retains the overlay and dirties the child. Only source 76 requests immediate child Update; later minimap composition/paint is a separate boundary.
- Added §4.5 and conformance item 49: layer 3→2 entry fixture, exact four pixels, dirty/clock deltas and unchanged pending sequence.
- Moved DS29-1 to docs/specs/questions/answered/map-view.md (DIFFERS); regenerated docs/specs/QUESTIONS.md. QV15 is untouched and remains open.
- Checks passed: questions.py check; ignored build-runs/check-re-ds29.py arithmetic/order/bookkeeping fixture; git diff --check.
- Evidence: main-checkout export/decomp plus instruction bytes. REA was not needed. No original implementation text is included in the spec.
- No docs/abi edits or proposed ABI rows. No engine files read or edited; engine changes in ancestry come solely from the explicitly requested drawshadow29 merge.
- Static RE only: no game run, pilot verdict, shadow count, subsystem verification or complete-method credit is asserted.
- Detached-HEAD commits succeeded; Git also printed a denied shared packed-refs.lock attempt during commit. Shared refs were not manually changed.

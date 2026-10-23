| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-propdev-pv14: prop developer owner closure | Codex / re-propdev-pv14 (detached HEAD) | ready: PV14 answered; doccheck passed | Implement read-only RNG snapshots or real-return transcripts; measure shadow equivalence. |

## Report

PV14 is answered in `docs/specs/prop-developer.md` §5.1–§5.3, with eight prose conformance vectors in §12.15–§12.22.
The answer supplies a version-guarded, read-only observation contract rather than requiring forced native seeds:

- Member/wealth generators: complete 624-word state, relocated cursor index, remaining count, quiescent-boundary invariants and shared-generator timing.
- CRT: original calling thread's 32-bit state at runtime-data +`0x14`, observable during its genuine call; allocation through `__getptd` is deliberately excluded as an observer accessor.
- Alternative: actual random-return transcripts; capture custom selectors once at both kept-check and creation call sites, preserving zero returns, candidate order, coordinates and requester identity.
- Exactness requires complete matched inputs and claimed outputs; counts/records alone do not certify random removals or creations. No exact-through-2032 claim is made.

### Evidence and validation

Commit `d4d7f4a1` contains the answer, archive entry and regenerated question index. The final report commit also clarifies that hardware execution observation can supply transcripts without native code/vtable patches.
Source: read-only `tools/re_query.py` export queries, including `asm --bytes`, and bounded class decompilation reads. No REA invocation was needed; the supplied session has no binary open. Numeric checks cover the `0x9C8` layout, unsigned high-product scaling, thread-state update and both selector call/return ABIs. The executable identity guard uses the measured SHA-256 of main's read-only `bin/SimCity4.exe`.

- `python tools/questions.py index`: regenerated successfully; PV14 archived as **ANSWERED 2026-10-23 (re-propdev-pv14): DIFFERS**.
- `python tools/questions.py check`: PASS.
- `python tools/doccheck.py check`: PASS.
- Eight conformance vectors are spec fixtures, not executed engine conformance or live shadow measurements.
- Cargo, clippy, CTest and game: n/a for this spec-only lane. No run labels requested or produced.
- No engine, plugin, capi, ABI tables/exports or coverage map read/edited. No proposed ABI rows required.

### Integration note

PV14 and the blocked owner report existed only in `glue-weather-propdev-shadow5`'s reserved worktree when this lane started. Those explicitly requested documents were read there; no engine source was read. The report has Findings/Next steps rather than an Owner closure table and names PV14 as its sole blocker. The full question is preserved in this lane's archive, with an empty open-question file. When landing the owner's filing commit, retain the archived answer and do not reintroduce PV14 as open; regenerate the questions index after reconciling both lanes.

Git emitted a `packed-refs.lock` permission diagnostic during the first commit; the detached-HEAD commit itself succeeded and the subsequent status was clean. No shared Git configuration/refs were changed or maintenance retried.

## Handoff

- Goal: answer PV14 for glue-weather-propdev-shadow5 with prose rules and conformance cases.
- Done: `d4d7f4a1` specifies/archives PV14 and regenerates the index; questions/doccheck passed.
- Worktree reused at detached HEAD; no engine files read or edited.
- PV14 read from the explicitly requested owner worktree's question/report; absent in this checkout/main.
- Owner report has Findings/Next steps rather than an Owner closure table; PV14 is its sole stated blocker.
- Spec §5.1–§5.3 now defines guarded snapshots and real-return transcripts; §12.15–§12.22 has eight prose vectors.
- Source verified using export asm --bytes and bounded decomp reads; REA not used (no binary open).
- Next: coordinator lands docs and resolves any duplicate open filing from the owner; implementer consumes §5.1–§5.3 and measures the completed shadow.
- Layout: 624 u32 words, pointer +0x9C0, remaining +0x9C4; total 0x9C8, cursor + remaining = 624.
- Native selector call/return: kept 0x006F4677/0x006F467A; create 0x006F1804/0x006F1807.
- Original CRT pre-draw: 0x009EFBA4, EAX runtime-data pointer, state +0x14.
- Source main: C:/Users/tlori/FreeCity4; export queries via tools/re_query.py.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-propdev-pv14-r1-manifest.json.

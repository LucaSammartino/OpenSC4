| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-draw-context-dc9 (TurnOnFog contract) | `Codex` / `re-draw-context-dc9` (RE lane) | ready; DC9 answered in `dc490e46`; nine spec vectors DC-V27–DC-V35; doccheck and questions check pass; ABI unchanged; game n/a | Engine lane: implement fog from §1.5.1 and §13.3. |

## Report

Answered DC9 in [draw-context.md](../specs/draw-context.md) §1.5.1, with a constructor-cache
cross-reference in §1.3. The contract specifies the native void thiscall signature, four arguments,
renderer field sources, driver slots and arguments, ordered cache updates, floating equality,
ordinary returned failures and retry suppression. The question moved to
[the answered archive](../specs/questions/answered/draw-context.md), and the index was regenerated.

The instruction audit establishes that enabling calls slot 44 with 6, then slot 31 with (0,2),
before caching enable. RGB, start and end each update after their own slot-30 calls. Return values
do not prevent later calls or cache updates. Colour forwards RGBA but caches only RGB; its fourth
cache remains untouched. Scalar maximum finite is leave-unchanged, signed zeros compare equal,
and NaNs resubmit. The end cache stores the end argument, correcting an erroneous pointer store
in the available pseudocode. All four direct renderer callers were checked.

Added nine independent expected conformance vectors DC-V27–DC-V35 in §13.3. They cover initial
sentinels, full call ordering, alpha-only changes, disable/re-enable, disabled parameter submissions,
reversed ranges, returned failures, signed zeros, NaNs and noncanonical nonzero enable bytes.
These are spec vectors; no replacement or game test was run.

Validation: `python tools/questions.py check`, `python tools/doccheck.py check`, and
`git diff --check` pass. No REA was needed; export instructions and existing decompilation were
available. Executable constant verification found `0x00ABB4D0` = binary32 bits `0x7F7FFFFF`.
No C ABI exports changed and no ABI rows are proposed.

The optional engine Owner closure table was not inspected because this lane explicitly forbids
reading engine/. No engine, plugin, capi or coverage-map files were read or edited. DC3's native
stack-mutation residual remains open; this lane makes no additional owner-closure claim.

## Handoff

- Goal: answer DC9 with signature, sources, cache order, driver calls and retries.
- Done: `dc490e46` publishes §1.5.1, DC-V27–DC-V35 and archived DC9; checks pass.
- Next: coordinator lands this RE result; engine lane consumes spec and adds executable vectors.
- Facts: native void thiscall, 16 argument bytes; enable/RGB/start/end caches update after calls; failures ignored.
- Evidence: 0x007D32D0 instruction pages and four renderer callers; constant 0x00ABB4D0=0x7F7FFFFF.
- Decompiler's final pointer store is wrong; bytes at 0x007D33E2–0x007D33E6 cache end argument.
- Do not read/edit engine; optional owner-table inspection conflicts with explicit prohibition and is skipped.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-draw-context-dc9-r1-manifest.json.

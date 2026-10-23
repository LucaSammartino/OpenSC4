| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-strings5 (framework strings and detokenizer) | Codex / `re-strings5` (RE lane; reserved detached worktree) | ready; behavior spec and 39 declarative vectors; doccheck passed; RZS1–RZS5 filed; no ABI exports changed; no game run | Implement settled contracts; resolve remaining native transform/callback edge questions as needed. |

## Report

Created [rz-strings.md](../specs/rz-strings.md) for cRZString and the base
cSCStringDetokenizer. It inventories all 29 string slots and 15 detokenizer slots,
native helper entry points, defaults/layout and reference ownership, synchronous
dispatch, counted edits, comparison/search behavior, parser escapes, callback
precedence, HTML cleaning, resource IO and partial-failure output. Neither owner
has a drawing or message interface. Linked language utility, language manager,
string encoding, LTEXT, crime-name loader, variant storage and existing ABI facts;
updated strings-language.md to point to the new contract.

Added 39 declarative vectors in the spec (RZ01–RZ20, DT01–DT19). No engine files
were read or changed. No executable replacement or in-game validation was run.
This is instruction/export evidence, not a measured engine conformance verdict.

Notable verified compatibility details: counted length survives interior NULs,
but sensitive virtual equality uses NUL-stopping comparison; Replace's count
erases old bytes and the replacement is scanned to NUL; unknown callbacks are
ordered/deduplicated by function address; false conversion retains partial output;
HTML cleaning affects replacements only; the first original hash position remains
an opening position after ## protection, so ### resolves an empty token.

Filed [RZS1–RZS5](../specs/questions/rz-strings.md): callback reentrancy, remaining
non-ASCII case mapping/integration, alias/allocation edges, native split variants,
and custom formatter edges. Regenerated the question index. Existing open language
questions were not changed. No proposed ABI rows or C ABI exports changed.

Validation: `python tools/doccheck.py check`, `links`, `wall` and `stale` passed.
Cargo/clippy/ctest are n/a for this docs-only RE lane. Used local re_query.py,
bounded decomp reads and instruction-byte checks; REA was not queried because the
brief says no binary is open. The q.py wrapper failed once with Git Bash
CreateFileMapping permission denied; subsequent reads used capped PowerShell.
Commits succeeded on detached HEAD despite Git's nonfatal packed-refs.lock
permission warning; no shared refs were changed.

## Handoff
- Goal complete for this research round; unresolved edges are explicit RZS1–RZS5.
- Done: 6cce37f9 checkpoint; 4fd34cd4 spec/questions/vectors; final report and cross-links follow in this commit.
- Next: coordinator publishes actual detached HEAD to reserved lane branch and lands ready queue entry; engine lane implements settled vectors.
- Facts: cRZString vtable 0x00A80810; detokenizer base 0x00AA6DFC. First-hash exception verified 0x0060096F–0x00600985; NUL restoration 0x00600BB0–0x00600BC1.
- Evidence helper build-runs/inspect-strings.py reads only the main executable's specified table/constant bytes; untracked/ignored and not committed.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-strings5-r1-manifest.json.

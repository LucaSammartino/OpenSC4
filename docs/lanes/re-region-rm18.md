| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-region-rm18: region owner questions | Codex / re-region-rm18 (detached HEAD) | ready; RM15–RM18 and RQ3 answered; doccheck passed | Land specs; core-region10 must change RM18 collision handling |

## Report

Extended [region-model.md](../specs/region-model.md) §§3.1, 5.5, 6.4, 7.1, 7.3,
7.4, 7.6 and 8.1 with the five owner-closure answers and added §12.1 conformance
vectors. Corrected the conflicting older summaries in [region.md](../specs/region.md)
§§6–7. Archived RM15–RM18 and RQ3 in questions/answered/region-model.md and regenerated
the question index. No implementation, ABI, coverage-map or game files changed.

Findings:

- RM15 MATCHES: parse up to eight hex digits after whitespace/optional 0x; ignore the
  remaining text. Zero preserves the complete literal name.
- RM16 MATCHES the proposed portable choice: store every type bit, but height lookup
  outside 0..1 reports Unsupported::BaseTerrainType. Original assembly reads unchecked
  process memory; type 2 addresses the global Init flag, not a third height.
- RM17 MATCHES the proposed safe adapter contract: retain invalidated opaque IDs as
  stale; resolution is None, with no accidental revival through storage reuse. A failed
  tutorial reload skips the ID write but still reactivates after successful export. The
  original actually dereferences freed storage and has no guaranteed outcome there.
- RM18 DIFFERS: suffix insertion uses the last dot in the whole resolved path, including
  parent directories. If there is no dot, collision insertion raises basic_string
  out-of-range (0x0090E8BF–0x0090E8C5 → 0x00407C40 → 0x009DD10A). It does not append.
  The portable adapter must expose filename-position failure. The spec includes 256-byte
  truncation and 260-byte suffixed-name vectors, reuse exemptions and directory-dot cases.
- RQ3 MATCHES region-model: folder suffixes start at 2; MoveCity fails on rename or
  post-rename read failure, ignores write and Init results; tutorial reset returns true
  on export failure, false on success/no match.

Validation: `python tools/questions.py index`, `python tools/questions.py check`,
`python tools/doccheck.py check`, and `git diff --check` passed. This is spec-only work:
cargo/clippy/ctest and game/shadow runs are n/a. Vectors are derived contracts, not executed
game observations. All new evidence came from re_query.py assembly and the exported
decompilation; REA was not queried because the brief says no binary is open.

Integration note: these five source entries were absent from this base and were read from
the reserved core-region10 worktree. RQ3 was actually filed there in questions/region.md.
They are archived together here as requested. If core-region10 lands its pending questions
after this change, remove those five open copies and regenerate the index; preserve this
archive. Other RM/RQ questions remain open. No new questions filed; no ABI-row proposal.

The first commit is `5d3dfc32d` (answers, vectors, archive and index). Git printed a
packed-refs.lock permission error during commit, then successfully committed detached
HEAD; no refs/config were altered. q.py failed with Windows CreateFileMapping error 5,
so capped native PowerShell commands were used.

## Handoff

- Goal: resolve core-region10's RM15–RM18 and RQ3 with original behaviour and vectors.
- Done: `5d3dfc32d` answers, vectors, archive and index; question check/doccheck passed.
- Next: coordinator lands detached tip; core-region10 adjusts RM18 and closes owner-table rows.
- Main export: C:/Users/tlori/FreeCity4; use re_query.py and decomp only, no REA.
- q.py Bash wrapper fails with CreateFileMapping error 5; use native PowerShell with capped output.
- No engine source inspection; only the explicitly requested Owner closure table.
- RM18 differs: dotless resolved path raises basic_string out-of-range on collision, never appends.
- RM16/RM17 adapter choices are explicitly distinguished from unsafe original reads.
- RQ3 was filed in core-region10 questions/region.md; archived in region-model as requested.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-scres-scr11: physical block comments | Codex, `re-scres-scr11` (reserved branch, detached HEAD) | ready: SCR11 answered; doccheck PASS | Implement section12, then measure strict raw replay through 2032. |

## Report

Answered SCR11 in [sc-resources section12](../specs/sc-resources.md#12-physical-block-comment-preprocessing-scr11),
with links from sections4.2/11.2 and 16 normative vectors SCR-V93–V108.
The source question was copied from glue-scres-replay7, then moved intact to
[answered/sc-resources.md](../specs/questions/answered/sc-resources.md), marked
DIFFERS from its current line-comment-only choice. The questions index was
regenerated: sc-resources has zero open and eleven answered entries.

The complete rule is a per-file, initially inactive, non-nesting flag. Exact
substring `#<` activates it and erases the line tail from the opener onward;
only then does active-state processing search for exact `#>` and erase the
prefix through that closer. With no surviving closer the whole line is
suppressed. Consequently a normal same-line opener/closer does not close,
but a closer preceding an opener can close on that same line. Quotes, groups
and ordinary line-comment text do not protect markers. Active EOF succeeds,
physical reads still count, and recursive/registered files start inactive.
Closing-line suffixes route normally, including an empty suffix or include.

Evidence: export queries and bounded reads of main-checkout cFileParser
decompilation; verified instruction bytes at `0x00680F59`–`0x00680FE0`, string
find/erase helpers, routing at `0x006811BA`–`0x006811CD`, and EOF/count cleanup.
Read-only PE inspection resolved pointers `0x00B0D10C`/`0x00B0D110` to the
literal marker bytes at `0x00AAF3A0`/`0x00AAF39C`. No REA was needed or queried.
The supplied glue lane file contained a final report rather than an Owner
closure table/Handoff; its remaining blocker was precisely SCR11.

Validation: `python tools/doccheck.py check` PASS; `git diff --check` PASS
after correcting a trailing blank line. Questions index generation succeeded.
Cargo, clippy and CTest n/a for spec-only work. No engine, plugin or capi
source was read or changed; no ABI, coverage-map or tool source changed.
No proposed ABI rows or exports.
No new game runs or replay equality claimed. Prior glue runs ash/birch
observed 45 suppressed reads among1107 true reads and72 strict raw mismatches;
these remain prior evidence, separate from the new normative vectors.

Implementation should retain physical read/position/count observations while
applying this filter before tokenization. Then rerun strict raw replay under
`scres_file_shadow` through2032 before crediting equality or changing its
default-off/house-profile status. No request remains outstanding here.
Git Bash q.py failed with Win32 CreateFileMapping error5; bounded PowerShell
queries were used. Git emitted a packed-refs.lock permission diagnostic but
the detached-HEAD commits succeeded; no shared refs/config were changed.

## Handoff

- Goal complete: SCR11 contract and vectors published, question moved, index regenerated.
- Milestones: `5a9e73477` imports source question; `a4734fb32` answers it with section12 and16 vectors.
- Next: coordinator lands docs; engine implements section12 and measures fresh strict raw replay.
- No new questions, game requests or ABI rows; no runtime equality inferred.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-renderprops-rp7 (render properties owner closures) | Codex / `re-renderprops-rp7` (RE lane, detached HEAD) | ready: RP7 answered; eight prose conformance vectors; questions check and doccheck PASS; ABI unchanged; game n/a | Land spec; reconcile shadow9's concurrent RP7 open entry with this archive. |

## Report

Answered **RP7** in [render-properties.md](../specs/render-properties.md) §1.5 and §7.4.
The eighteen public owner slots expose current state, not an ordered input journal. The
view has no public snapshot getter: its second properties object is embedded at complete
view +0x380 and serves as a scroll snapshot. The view text helper writes the application.
Enabled scroll begin saves the application tables before four Boolean writes and one
float write; enabled scroll end restores the saved tables without calling setters.
Both paths test Boolean 30 (`kNoPartialBackingStoreCopies`) independently, and literally
copy P+8, the reference-count field, too. These quirks are now explicit compatibility rules.

The diagnostic protocol specifies typed setter arguments/completions, instance generations,
snapshot-copy relationships, complete-history requirements and unavailable/gap handling.
It prohibits native output bootstrapping and destructive lifetime tests on borrowed live
objects. It is adapter policy, not an existing native subscription or implemented bridge.
The SCRes/weather DR0 loan cannot provide continuous setter coverage inside parser frames;
a coordinated independent trace facility is required or observation stays unavailable.
Registry-only credit does not become whole-owner credit from this spec change.

Added **eight instruction-derived prose conformance vectors/recipes**: duplicate writes,
raw finite float/integer arguments, ordinary scroll save/restore, disabled paths, enable
flag changes, text-helper target, missing-history handling, and observer/lifetime failure.
None is an executed native vector or game verdict. No engine, plugin, capi, coverage-map
or ABI files were read/changed for implementation; no ABI rows are proposed, no C exports
changed, and no game run was requested or remains outstanding.

Evidence came from main-checkout `tools/re_query.py` first and bounded reads of the exported
decompilation; numeric/layout claims were checked against instruction bytes. Principal
addresses: setters `007DDE60/80/A0`; construction `007F0E32–007F0E70`; scroll begin
`007F6410`, end `007F6160`; table-copy helpers `007F5F00`, `007F6010`; public view vtable
`00ABCE40` and QI `007E5630`; application getter `0044B660`; text helper `007ECDA0`;
rules property-set application `007CE1F0`. REA was not called, as instructed.

RP7 was absent from this reserved base and main but present in sibling shadow9's question
file. Its original text/provenance is retained in the answered archive, marked
**ANSWERED 2026-10-24 (re-renderprops-rp7): DIFFERS**. The supplied shadow9 report contains
no Owner closure table; its explicit uncompared-owner list supplied the requested scope.
The regenerated index has render-properties **3 open / 4 answered / next RP8**; RP1, RP2
and RP6 remain open. When landing shadow9 too, remove its concurrent open RP7 entry rather
than retaining both open and answered copies, then regenerate the questions index.

Validation: `python tools/questions.py index` completed; `python tools/questions.py check`
PASS; `python tools/doccheck.py check` PASS; `git diff --check` PASS. Cargo/clippy/CTest
are n/a for this spec-only lane. Initial q.py failed with Git Bash CreateFileMapping
Win32 error 5; bounded PowerShell commands succeeded. Git reported an unwritable shared
packed-refs lock during commits but created the detached-HEAD commits successfully; no
shared refs/config were changed by this lane.

## Handoff
- Goal complete: RP7 behavior, observation limits and eight conformance recipes delivered.
- Milestones: `192bccb70` scope/evidence handoff; `9e5597116` spec, archive and index, with questions/doccheck PASS.
- Next: coordinator lands this detached tip and reconciles shadow9's concurrent open RP7; no branch switch or merge performed.
- Implementation follow-up: independent trace/private view bridge or isolated owned fixtures are needed for omitted owner-method credit; this is not a remaining RE question.
- No game requests, ABI changes, external RE output or engine edits.

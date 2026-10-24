| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-widgettk-wt13: window and text-edit questions | Codex / detached re-widgettk-wt13 | ready: six requested answers plus WT5 and TE9; doccheck passed | Land specs and apply corrected contracts in core-uiwin-close11 and core-widgets-close12 |

## Report

Resolved WT10, WT11, WT12 in `docs/specs/widget-toolkit.md` and TE10,
TE11, TE12 in `docs/specs/text-edit.md`. Added 15 window and 5 text-edit
conformance vectors derived from the native instructions, not game runs.
Archived answers in the respective spec question archives. The requested
questions were retrieved with git show from the two engine lane branches;
they were not yet present in this worktree's open question files. The same
evidence also resolved and removed existing WT5 and TE9. Regenerated
`docs/specs/QUESTIONS.md`: 605 open / 1491 answered across the repository.

Corrections implementers need:

- WT10: live filter iteration exposes same-pass additions. Removals retain
  owning pending entries; only an outer PlotComposite filter pass drains
  them. DoMessage never drains them. Repeated queued removal without a
  remaining registration reaches an original sentinel-erasure defect.
- WT11: type 1 propagates post-ChildDelete AL, including the base manager
  result; it does not force false. Custom void overrides require an explicit
  byte observation to reproduce this return channel.
- WT12: constructor count zero; QI success stores primary pointer and
  AddRefs, failure preserves output. AddRef wraps; unsigned Release at
  zero or one invokes deleting slot 148(1) without decrementing first.
  Busy manager requests queue raw pointers without detaching or owning;
  immediate requests shut down and release rather than forcibly freeing
  retained objects. Old COW references delay destruction/unregistration.
  Direct parenting lacks self/cycle/validity checks. Valid tree domain and
  unsafe callback/lifetime boundaries are explicit.
- TE10: three color words are 0xFF000000, horizontal offsets and visible
  metrics zero, initial line height 1, image option true. Sound IDs at
  W+0x27C..+0x288 are uninitialized, not zero. TE9's preferred X is
  0xFFFFFFFF and maximum undo levels 0x4000.
- TE11: shared signed counter <1, increment/check/decrement only on normal
  return from RecalculateAll's automatic check; no exception-safe reset.
- TE12: false direct / true edge; edge mode 1 divides source right/bottom
  by signed 3 and excludes scrollbar strips. Direct uses full draw area.
- WT5: nonempty ChildDeleteAll under a persistently busy manager can loop
  indefinitely because queuing leaves the front child attached.

Evidence used the main checkout's read-only re_query export and bounded
decomp reads. Numeric widths, comparisons, constants and dispatch effects
were checked against asm --bytes and cited in the specs. No REA calls were
needed; no REA errors or fallback results. No decompiled source or assembly
listings were copied into specs. No engine/, plugin, capi, tools or ABI
table edits. The requested owner closure tables reside under engine/;
the explicit prohibition on reading that tree took precedence. Lane
reports and exact filed questions supplied the closure targets instead.
The unpublished base vtable WT1 and unrelated parked text helper questions
remain outside this work; no full native ABI or widget-family closure claim.

Validation: `python tools/doccheck.py check` passed. Questions index
generation succeeded. Cargo, clippy and ctest: n/a (spec-only lane).
Game runs: n/a; shadow comparison: n/a; no outstanding requests.
New questions: none. C ABI exports changed: no. Proposed ABI rows: none.

## Handoff

- Goal complete: WT10–12 and TE10–12 answered; WT5/TE9 incidentally closed.
- f6b63ceee: initial lane record.
- 7d794b447: text-edit defaults, reflow and image mode answers/vectors.
- de92b4a29: window filter, return and lifetime answers/vectors; archives.
- Final report commit also records regenerated questions index and final gate.
- Next: coordinator lands this detached HEAD using reserved lane publication;
  engine owners apply the DIFFERS rules above and validate against vectors.
- No game runs, ABI proposals or new questions pending.
- Queue: build-runs/merge-queue/re-widgettk-wt13.json, kind re, full HEAD tip.
- Native evidence retained only in main checkout; no external artifacts committed.

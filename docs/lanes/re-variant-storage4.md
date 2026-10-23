| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-variant-storage4: native variant storage/lifetime | Codex / re-variant-storage4 (detached worker, RE docs) | ready: bounded static publication; no native qualification | Land docs; HG6 answered, HG5 partial; preserve verifier 3/3 parked |

## Delivery

Published [variant-storage](../specs/variant-storage.md), linked from foundations §§9.1/17.1.
This is FIRST dedicated storage/lifetime research, one round of this invocation, not a
fourth verifier round or another exemplar conversion phase. READY means a coherent bounded
specification only. No whole-owner/native/default/byte qualification is claimed.

Milestones: `ec74eb84` establishes original identity/default/Erase/clone/alias rules;
`4243904e` adds CopyFrom/self-copy, family width consistency, strings/objects/pointers,
question disposition and independent static vectors. Final review adds direct containing-class
constructor evidence, explicit foundations return/ownership rules and lifetime wording fixes.

## Findings and measured source facts

- Original PE SHA-256 matches export summary:
  `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`, base `00400000`.
  Canonical main-checkout re_query/export was queried first; existing class decompilation
  located functions; original instruction bytes decided widths, branches, order and channels.
  No REA RPC was necessary; no live readiness claim, retry or server change was made.
- Original cSCProperty constructor embeds cRZVariant at +4 and installs AA6788. Standalone
  default/copy constructors install ACECD0. The first 127 table entries differ only at
  identity/ref slots0/1/2; each table has 85 distinct entry addresses. These counts are ABI
  inventory, not unique-body/byte/behavior coverage. Folded accessors, tail wrappers,
  misleading foreign labels and foreign primitives are distinguished in the spec.
- Standalone clone construction clears a 16-bit own reference count, then CreateCopy calls
  AddRef and publishes one owned reference. Erase/CopyFrom/CreateCopy explicitly set AL=true
  on normal return; allocation-null branches do not establish clean false/null recovery.
- Ref input setters copy bytes; matching type/count plus non-null pointers reuses storage.
  Replacement disposes old storage before allocation/copy and publishes count before type.
  All 16 finite Ref input bodies were inspected for type, multiplier and owner call structure.
- Mutable numeric/character As aliases point into inline storage; both AsVoidPtr slots load
  the pointer instead. Raw Ref getters load the stored data word. Alias expiry, exact self
  copy, no snapshot on replacement, distinct array copy and shared held-object ownership
  are specified without invalid-memory experiments or blanket self-copy no-op assumptions.
- SetValString uses length then pointer and copies exact bytes, including embedded NULs.
  Original output cRZString primitives are qualified only for finite lengths0/1/3. CreateValString
  publishes one owned reference on success; getter-false releases the temporary and clears
  output. General string growth/overlap/failure and foreign output behavior remain partial.
- Held-object replacement AddRefs new then Releases old before storing new when already
  in object mode; array-to-object disposes array first. Same pointer is silent. Raw getters
  are borrowed; query getters delegate ownership/output policy to the held object, while
  owner-handled wrong-type/null failures preserve output. VoidPtr never owns its pointee.

## Validation and boundaries

Question index regenerated; `python tools/questions.py check`: **ok**. Local documentation
check: two links from the new spec resolve, 16 unique VS-A–VS-P vectors marked UNEXECUTED,
HG6 appears once in the archive and is absent from open questions, HG5 remains open, and
the behavior spec contains no code block. `git diff --check`: clean. All tracked changes
are documentation. No cargo/clippy/ctest/build/test/game/native execution was requested or
run; gate and game are n/a. No C ABI exports, ABI rows/tables/version, coverage map or shared
lane/coordinator state changed. Git commits completed despite shared packed-refs lock
permission messages; no ref/config repair, maintenance or branch switch was attempted.

The 16 independent literal traces cover external mutation, lengths0/1/3, null only on
proved owner branches, mutable alias write-through/reuse/expiry, scalar→array→string→object,
self and distinct copying, clone mutation isolation, embedded NULs, failure sentinels and
precise conditional object-reference changes. They are static proposals, not measured
native passes or a new self-oracle. No verifier source or implementation output was read.

Admission consumed the immutable copied eligibility/full GO host proof and current own
manifest (lane re-variant-storage4, kind re, round1, this worktree, dependencies
verify-variants4/re-exemplar-final-gaps4, base e8c849c6, 150000/100/maxRounds1).
No coordinator global Git/npm/spawn preflight was rerun. Runtime guard remained within
budget at milestones. No logical-family algorithm overlap was researched: published
conversion/parser/property facts were inputs, and cSCProperty was followed only for
embedded identity construction. No external messages or subagents were used.

## Questions and next steps

HG6 moved to [answered/foundations](../specs/questions/answered/foundations.md), marked
ANSWERED 2026-10-23 (re-variant-storage4): DIFFERS, with rules also in foundations §17.1.
HG5 received a partial-publication link in [open foundations questions](../specs/questions/foundations.md)
and remains open. No new question IDs were filed.

Land the bounded docs through the normal own merge-queue entry. Preserve explicit unknown
allocation/string/foreign-interface/reentrancy/non-finite domains. There is no authorized
automatic restart, verifier promotion or additional round. verify-variants4 stays used3/3
parked; its historical 28 HG7 differences and 70cases/420checks remain unchanged. Prior
three exemplar RE phases stay capped, HG7 stays answered, and HG4 remains outside scope.

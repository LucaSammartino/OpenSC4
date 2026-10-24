| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotrec-lr13: construction owner closure | Codex / `re-lotrec-lr13` (RE lane, detached HEAD) | ready: LR13 answered; eight vectors T66–T73; doccheck/questions pass; no ABI change or game runs | Coordinator integrate LR13 archive with ctor7's incoming question; implementers apply §9.1.1 and T66–T73. |

## Report

Added lot-records §9.1.1 describing fields 10–13/15 writer-facing views, Serializable QI identity,
callback retention, read acquisition ownership and failed-IID behavior. Both effect views resolve
to the derived effect Serializable. A rejected reference becomes null; construction Read can
still succeed because it ignores the helper result. Such a record is a mismatch. Construction
Write does not substitute temporary occupants; effect Write's temporary empty string is distinct.
Eight normative conformance vectors cover aliases, props/crew, nulls, rejected interfaces,
callback failure ownership, QI balancing and repeated writes. They are specifications, not executed tests.

Evidence was queried first through main/tools/re_query.py, then checked in main/decomp and paged
assembly with bytes. Key paths: Write 005CD250, Read 005CE770, helpers 005D99A0/005D9730/005D96C0,
effect constructor 005F0210, effect QI 005ECC10 and adjustors 005F02C0/005F02F0. No REA used or needed.
No engine, plugin, capi, tools/coverage-map.tsv or ABI tables were read/edited as implementation material;
ABI vtable rows were read only to establish derived interface identities. No proposed ABI rows needed.

Validation: questions index regenerated; questions check PASS; doccheck check PASS; git diff --check PASS.
Cargo/clippy/CTest n/a for this prose-only RE lane. No game requests or new game measurements.
The ctor7 report's 563/563 result is upstream context, not this lane's test result. The historical
254/250-byte train180 mismatch is not reproduced or diagnosed here and remains a runtime investigation.

Integration: main/base had no LR13 question or ctor7 lane report. The question was recovered verbatim
from the ready ctor7 worktree and archived as answered; there was no local open LR13 entry to delete.
If ctor7 is merged later and adds the open entry, remove that duplicate in favor of this archived
answer and regenerate the index. LR6/LR7/LR12 already present in the local open file were outside
this LR13 task and are unchanged. No new questions filed. The referenced ctor7 report has no Owner
closure table or `open:` rows; its Construction limit and question section names LR13 explicitly.

Milestone: b28a5ddfc committed the spec, vectors, archive and generated index. Git emitted a denied
packed-refs.lock housekeeping message but the detached commit completed successfully; HEAD was verified.

## Handoff

- Goal complete: LR13 contract answered in §9.1.1, vectors T66–T73, archived answer and index.
- Done: b28a5ddfc spec milestone; doccheck/questions/diff checks pass.
- Next: supervisor publish detached commits; reconcile any incoming open LR13 from ctor7.
- Runtime historical failure is not claimed fixed; this lane supplies the native contract for investigation.
- No ABI change, proposed rows, game requests, engine edits or new question IDs.

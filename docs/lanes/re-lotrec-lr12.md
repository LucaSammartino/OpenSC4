| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotrec-lr12: lot record inventory | Codex / re-lotrec-lr12 (reserved detached HEAD) | ready: LR12 answered; doccheck passed | Implement full master-list record shadow per lot-records §14.1; no runtime coverage claimed. |

## Report

Answered LR12 in [lot-records.md](../specs/lot-records.md) §14.1 and added eight normative conformance
vectors T58–T65 in §17.6. The scope is explicitly a FreeCity4 choice: every current master-list lot's
complete 27-field payload, including inline trips/properties, rather than every class in the save-format
catalogue. Off-list lots, business units and occupants are excluded as payloads; references retain
identity and aliases without recursive discovery. Unspecified prop subclasses cannot be reported as
successfully checked. This resolves the scope ambiguity without claiming all-class coverage.

Specified city/manager acquisition, count and safe-iterator signatures, borrowed versus acquired
references, nonvirtual iterator cleanup, Serializable QI/Write, exact consumption and complete comparison,
empty inventory and explicit failure outcomes. Vectors discriminate multi-cell membership, off-list objects,
shared/cyclic references, empty versus unavailable owners, partial Write failure, nested differences,
duplicate/null entries, count changes, identity mismatch and trailing bytes.

Native evidence was queried from the main checkout with `re_query.py` before any decomp lookup; no
decomp or REA was needed. Instruction checks (including bytes) covered city getter `0x00472E70`,
manager count `0x00809250`, iterator acquisition `0x00734850/0x00734860`, cleanup `0x0045D5B0`,
lot QI `0x006BF190` and Write `0x006C0080`. Existing `zones.md` §8.3.1 and foundations §2.7
supply the iterator/reference contracts. All numeric native facts added here were instruction-checked.
No ABI row changes or C ABI exports are proposed. No engine/, plugin, capi or coverage-map file was read or edited.

Validation: `python tools/questions.py index`, `python tools/questions.py check`,
`python tools/doccheck.py check` and `git diff --check` passed. Questions index now records 2 open,
10 answered, next LR13 for lot-records. Cargo/clippy/ctest: n/a (spec-only). Game runs: none;
load/month shadow exactness is unmeasured. Conformance vectors are specified cases, not executed results.

LR12's source entry existed only in the blocked engine lane's worktree, not this checkout or main.
It was copied verbatim into the answered archive and marked ANSWERED; there was no local open LR12
entry to remove. LR6/LR7 remain untouched. The engine lane's current report has no Owner closure
table or `open:` rows, so its explicit LR12 deferral paragraph and question were used. When landing
that engine branch, retain this answer and remove any incoming open LR12 duplicate.

## Handoff
- Spec/archive/index milestone: `28502cdad`.
- Goal complete: LR12 closed; implementation can proceed under the master-list-only scope.
- Next: coordinator lands spec, then engine lane implements §14.1 and exercises T58–T65 and live load/month captures.
- No game request, ABI proposal, external write or runtime coverage claim is outstanding.
- Preserve LR6/LR7; do not reintroduce the engine branch's archived LR12 question as open.
- Worktree remains at detached HEAD; supervisor publishes commits to reserved lane branch.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-effects-ef20 (native capture contracts) | Codex / `re-effects-ef20` (reserved lane; detached HEAD) | ready; EF20 answered; doccheck passed; no ABI exports changed | Land spec answer; glue-effects-shadow7 can implement the defined capture scope |

## Report

EF20 from glue-effects-shadow7 is answered in [effects.md](../specs/effects.md)
§18.7. The source question was copied from that lane's worktree into the
answered archive with its original wording and a dated DIFFERS answer.
The questions index was regenerated. Its current lane report has no Owner
closure table, `open:` rows or Handoff section; its landing restriction and
the EF20 text were used to identify the blocked scope.

The seven collection getters return bool in AL, not a defined full EAX.
Six assign to caller-constructed value objects; the component getter writes
an acquired object pointer, released through description slot 5. All named
getters change native scratch state. No collection count/enumeration API
exists; slot 23 is a deleting destructor. The spec supplies vector/hash
framing, index and duplicate-name preservation, anonymous descriptions,
bindings/triggers, both camera copies and derived lookup tables.

Resource IID CA51189B slot 5 is a void collection publication operation,
distinct from stream Read. Its empty-destination fast path exchanges native
storage; the spec explicitly excludes invoking it as a capture iterator.
The retained file queue has 0x2C-byte records and pure count/path/diagnostic/
modified accessors. A quiescent snapshot records retained values without
owner calls; natural-operation tracing records bootstrap history that final
state cannot recover. Names/counts alone never establish payload equivalence.

Evidence used the main checkout's re_query.py export first, then bounded
decompilation reads and instruction queries with bytes. A local read-only PE
helper confirmed resource vtable slots omitted from re_query's vtable index;
it is ignored under build-runs/read-vtables.py. No REA call was needed.
Address citations are in the spec. No decompiled text or listings were added
to specifications.

Validation: `python tools/doccheck.py check` passed; `git diff --check` passed.
Ten EF20 conformance vectors are specified as proposed and unexecuted.
Cargo, clippy and CTest are n/a for this documentation-only lane. No game
requests or runs; no shadow verdict or expanded owner coverage claimed.
No engine, plugin, capi, docs/abi or coverage-map edits; no proposed ABI rows
and no C ABI export changes.

Commits: `89a039de0` records the initial handoff; `679782d21` adds the EF20
contract, archive entry and regenerated index. Git printed a shared
packed-refs.lock permission diagnostic during commit housekeeping, but both
detached commits completed successfully; no shared refs were changed.

## Handoff

- Complete: EF20 spec contract, ten proposed vectors, archive and index.
- Next: coordinator lands detached tip and reconciles the source branch's still-open EF20 entry when landing that branch; glue-effects-shadow7 implements capture against §18.7.
- Keep snapshot equivalence, publication history and payload/render coverage separate. Full-owner equivalence needs its own implementation and measurement.
- No game request, question or validation operation outstanding.

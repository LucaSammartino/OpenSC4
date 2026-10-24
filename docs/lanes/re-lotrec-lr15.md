| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotrec-lr15 (lot owner closure LR15) | Codex / `re-lotrec-lr15` (RE lane, detached HEAD) | Ready: LR15 answered; nine conformance fixtures T80–T88; doccheck and questions check passed; no game runs; ABI unchanged | (1) Land with originating LR15 open entry removed; (2) engine implement and measure separately named scoped inventories. |

## Report

Resolved LR15 from glue-lots-shadow12's owner table and question file. Added lot-records §14.2,
cross-referenced from §§4/6/7, and nine behavioural conformance fixtures in §17.9. The original
master-lot/inline-trip scope in §14.1 is preserved.

- Business-unit GetUnit is slot 25, target 0x0068D1C0: nullable borrowed pointer with no AddRef.
  Serializable QI owns an independent reference. Deduplicate shared unit pointers across the full
  master-lot walk; capture every selected unit's complete ordered member list.
- The getter union cannot establish all allocated units: the native constructor/factory can create
  an empty unit. The chosen default-off lotunit_shadow is explicitly master-rooted, excluding
  empty/off-master-only units. It must not claim unconditional complete-owner verification.
- Textures/walls use manager slot 39, explicit inclusive world-grid corners (0,0)…(191,191),
  type 0x497F6D9D / 0x895D1169, maxCount=-1. This covers registered objects outside lot bounds
  and city terrain padding, with native duplicate suppression. Null corners are a smaller range.
- Each list element owns one reference; slot 48 releases elements and nodes, caller frees the
  allocator-A sentinel. QI owns another reference. A successful empty list is distinct from
  uninitialized/zero-maxCount failure. Malformed/truncated discovery is never exact.
- Base textures/walls are registered-grid inventories, not all allocations. Wall hash-induced
  mask growth remains an explicit round-trip mismatch. No new original-game behaviour is inferred
  from the observer's chosen names, scope or verdict rules.

### Evidence and validation

Read-only main-checkout re_query.py instruction queries verified getter, constructor/factory,
QI adjustments/acquisitions, typed query dispatch, grid clamp/inclusive traversal, append retains,
cleanup and AL return paths. Narrow decomp reads corroborated unit membership/Write behaviour;
existing occupant-manager §§8/12 provide the container contract. No REA call was made: the export
was sufficient, and no binary-open REA session was assumed.

`python build-runs/check-re-lotrec-lr15.py`: questions index regenerated; questions check PASS;
doccheck check PASS; git diff --check PASS. Logs: `build-runs/*-re-lotrec-lr15.log`.
No cargo/clippy/CTest or game run applies to this prose-only RE change; fixtures are requirements,
not executed conformance claims. No engine/plugin/capi/ABI/coverage-map edits or proposed ABI rows.
The first commit succeeded despite Git's subsequent packed-refs.lock permission diagnostic;
no shared-ref/config repair or maintenance was attempted.

### Questions and landing

LR15 is archived as ANSWERED DIFFERS in `docs/specs/questions/answered/lot-records.md`.
It was absent from this baseline's open file, so its originating text was copied directly to the
archive without adding an open copy. When landing with glue-lots-shadow12, the coordinator must
remove that lane's open LR15 entry rather than merge it back as open. LR6/LR7 are unchanged.
No game request is outstanding. Engine follow-up must report the scoped denominators explicitly;
these contracts alone do not close all-allocation replacement coverage.

## Handoff
- Goal: resolve LR15 in lot-records with behaviour and conformance vectors; no engine edits.
- Done: c13295c0f commits §14.2, §17.9 T80–T88, LR15 archive/index; checks passed.
- Next: coordinator lands final tip, removes originating open LR15, then engine consumes §14.2.
- Facts: unit slot 25 target 0068D1C0 borrows +4C; full-grid occupant slot 39 type-filtered, explicit (0,0)…(191,191), maxCount=-1; list owns references, slot 48 cleans nodes.
- Scope: units rooted in master lots only (not all allocations); textures/walls registered in full grid. No game runs or ABI changes.
- Evidence roots: main checkout export/decomp read-only; edits confined to this worktree.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-lotrec-lr15-r1-manifest.json`.

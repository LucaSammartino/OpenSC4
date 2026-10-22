| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-persist-manager4: cGZPersistResourceManager complete-owner spec | `re-persist-manager4` / (RE spec lane, detached worktree) | ready: 109/109 functions, 8,437/8,437 B audited; 30 proposed vectors; PR4 filed; doc checks pass; cargo/clippy/ctest n/a; no ABI or game runs | 1. Review/apply one exact-owner coverage-map proposal; 2. implement PM01–PM30; 3. resolve PR4 caller return policy |

## Report

Completed the manager contract in `docs/specs/persistence-manager.md`, linked as persistence §10.
The companion covers every selected body, including the built-in cache, type filter, container helpers
and lifecycle thunks. It adds 30 independent input/output/failure vectors PM01–PM30. These are proposed
conformance cases, not executed tests or game/shadow evidence.

Milestones: `cedc284d` committed initial dispatch/lookup rules; `a8bcf375` committed the full owner contract,
vectors, native factory-slot correction, PR4 and regenerated question index. The final documentation
commit adds the complete hash-growth constants, pointer preconditions and this report.

### Measured results

- Selection: 109 functions, 8,437 B from main's `build-runs/run4-selection-owners.tsv`. Largest bodies read
  first; truncated displays reread. Companion §11's function-start set matches exactly: no missing/extra.
- Loads cache before requested QI; explicit creates cache after QI. Failed loads retry on later calls,
  with no same-call fallback after the selected factory/segment fails.
- Factory/type registries retain duplicates; growth reverses equal-key chains and can change selection.
  Factory removal and Shutdown retain type mappings. Ordered segment movement and referenced/borrowed
  outputs are specified, as are key generation, filter changes, purge cadence and ownership.
- Writes return initial open/create status even after factory/Write/commit failure. Shutdown cleanup
  order and lack of equivalent destructor cleanup are explicit.
- Factory vtable `0x00AD8C20` corrected persistence §4.2's reversed overloads: slot 3 takes a record,
  slot 4 takes a type, slot 8 is the record convenience form. No ABI table edits.
- Native instructions verify widths/order, signed tick and unsigned purge thresholds, hash/equality,
  duplicate relinking, record flags, outputs and write status. Misleading folded labels were not used as
  semantic authority. Export/decomp supplied all bodies; no REA query/error/fallback was needed.

### Validation and questions

- `git diff --check`: pass. Audit sets exact at 109/8,437; all 30 vector IDs and companion link checked.
- `python tools/questions.py index`: regenerated; persistence row is 4 open, 0 answered, next PR5.
- PR4 in `docs/specs/questions/persistence.md` asks which shipped callers observe the service update
  return and which deterministic return an implementation should expose. Native `0x0097356D` returns
  an uninitialized OR byte. Callback effects are specified; no deterministic byte/caller policy guessed.
  Existing PR1–PR3 unchanged; no entries moved to answered.
- No engine/ reads or edits, tools/ABI table edits, or C ABI export changes. Runtime gate n/a for docs:
  cargo/clippy/ctest not run. No game requests, game verdicts, native test executions or shadow claims.
- Detached commits succeeded; Git also printed a denied common `packed-refs.lock` attempt. The supervisor
  publishes the reserved branch; this lane did not switch branches, merge, push or edit shared refs.

### Exact-owner proposal and next steps

One local proposal is `build-runs/coverage-map-re-persist-manager4.tsv`, in coverage-map format:
pattern `cGZPersistResourceManager`, subsystem `persistence and resources`, area `framework`, spec
`persistence.md`, core/shadow/replace all `-`. Coordinator should review and insert this exact row before
the generic `cGZPersist*` row. No family claim, ABI rows or runtime coverage promotion.

Persistence's owner table now totals 262 functions / 23,229 B including the earlier seven foundation
owners; this lane adds only 109 / 8,437. Implementers can build PM01–PM30 with instrumented interfaces
and reference ledgers. PR4 needs caller evidence before choosing an initialized return policy.

Navigation artifacts remain uncommitted under `build-runs/`: manager-inventory.tsv, audit-manager.py,
native-tables.py and bounded body extracts. No pseudocode, binaries or external data are committed.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-persist-manager-complete4: PR5 enumeration completion | `core-persist-manager-complete4` / detached reserved worktree (engine lane) | in progress; corrected core-only round 2/2; ABI unchanged | 1. Correct PR5 and test counterexamples. 2. Refresh owner audit preserving PR4 policy. 3. Focused tests, full gate and fmt. |

## Handoff

- Goal: complete published PR5 enumeration in core only; native prerequisite removed.
- Reused detached HEAD 8ae81c65; initial worktree clean.
- Done: PR5 enumeration copies one current key before either predicate and output;
  audit now records published PR5 and explicit PR4 uninitialized-byte deviation.
- Added three PR5 tests with exact primitive traces and ownership cleanup;
  A-D run with/without output, E checks duplicates/type filters, F both predicates.
- Next: finish focused core/conformance quick check, then stable full gate/fmt.
- First focused compile found a missing Clone derive on test events; corrected.
- Audit inventory has 109 rows; portable KeyList cannot model failing foreign append.
- Source: engine/core/src/persistence_manager/{cache,manager,test_host,table,primitives}.rs and AUDIT.md.
- Preserve PR4 initialized logical-OR policy and native/reentrancy limitations.
- No baseline gate per corrected task; focused and final checks follow substantive edits.
- No native binding, CAPI, plugin, game, shadow or replacement work authorized.
- No outstanding game labels or new questions.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-persist-manager-complete4-r2-manifest.json.
- Prior round report below is historical; its prerequisite is superseded.

## Result

Stopped at the brief's prerequisite check within 15 calls, before baseline builds.
This is a documentation-only blocker report, not a completed core correction or
native verification. Reserved detached HEAD was reused from `63cabe1b`; no refs,
shared tools, ABI tables, configuration, game files or engine sources changed.

## Published source and PR5

The parent report `docs/lanes/core-persist-manager4.md` identifies the actual
source as `engine/core/src/persistence_manager/`. Its native-integration limits
are corroborated by `primitives.rs` and `AUDIT.md`.

The answered contract is present in persistence-manager.md section 8.1 slot 12
and section 10.1a (PR5-A through PR5-F). Enumeration must obtain one current
resource key per visited entry before filtering, apply stored then supplied
filters to that copied key, and append that same copy. Stored lookup keys remain
unchanged after external SetKey. The production method in `cache.rs:157` still
filters `stored_key` and calls GetKey only for accepted entries with an output
list. Its PR5-open comment and the corresponding audit paragraph are stale.
They remain unchanged because the brief explicitly requires stopping when the
native binding prerequisite cannot be confirmed.

Required focused tests after unblocking: current versus stored key rejection and
acceptance for each predicate independently; unchanged stored-key lookup;
duplicate current output keys; null output still fetching/filtering current
keys; filter-triggered SetKey preserving this call's copied output; balanced
references. Published A-F are specification expectations, not executed native
measurements.

## Precise prerequisite gap

- `Primitives` represents foreign pointers as symbolic `Object(u64)` and lock
  scopes as events. A native embedding must supply actual interface operations,
  recursive locking and ownership. Anchored implementation search in core,
  conformance, C API and plugin finds only `test_host.rs:19` and conformance
  `tests/persistence_manager/support.rs:195` implementations.
- No `persistence_manager`, `fc4_persist_manager` or `FC4PersistManager` references
  were found in `engine/capi` or `engine/plugin`. There is no published adapter
  here calling the actual production owner from a native comparison harness.
- SDK `cIGZPersistResourceManager.h` forward-declares
  `cIGZPersistCacheStrategy` and exposes AddCacheStrategy/RemoveCacheStrategy,
  but its include directory contains no PersistCache interface header.
  Targeted ABI searches for that interface and the documented built-in cache
  vtable/enumeration entry yielded no rows. Manager ABI rows exist; they do not
  provide the missing complete cache invocation/construction contract.
- The spec supplies behavioral slots and ownership rules, but a supported
  callable native cache construction/acquisition interface, complete binding,
  storage and destruction adapter for independent raw-input tests was not
  confirmed. No address cast, guessed layout, native hook or speculative ABI
  was introduced to bypass that gap.

The next authorized implementation needs the supported binding and lifetime
contract for an independently retained original cache/resource/filter/list,
plus the production-core host adapter. Compare actual original native calls and
actual core methods on identical inputs with separate ownership; no second-Rust
oracle, seeded native outputs, private verifier algorithm or resynchronization.

## PR4 and owner-audit limits

Section 9.2 explicitly chooses initialized canonical logical OR with every cache
visited. This is a deviation from the undefined native initial byte, not exact
native return behavior for empty/all-false cases. The existing owner audit
already records initialized logical OR and explicitly disclaims native ABI/live
integration. Its 109-start mapping remains a source checklist, not runtime
coverage; PR5 correction and complete-audit refresh remain outstanding.

## Checks, questions and next steps

No baseline, focused tests, full gate, formatting checks or game requests were
run: the task says to stop before baseline expenditure if this prerequisite is
missing. No outstanding labels, native/own/current40 counts, 2032/ref0 verdict,
save/shutdown or supported reload evidence exist for this phase. Parent gate
counts are historical and are not claimed for this worktree.

No new question was filed: PR5 is answered and PR4's policy is explicit. The
remaining gap is an integration prerequisite, not a new behavior ambiguity.
Do not mark this lane ready or promote owner/native/replacement coverage.
No subagents, restart, RE inputs, external communication, merge, rebase or push.
Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-persist-manager-complete4-r1-manifest.json`.

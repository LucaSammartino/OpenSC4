| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-feature-manager4 (portable feature manager) | `Codex` / `core-feature-manager4` (engine lane; detached HEAD) | Ready: code 61844d96, vectors/frozen source 1df2f744; cargo 4340/0, clippy 0 diagnostics, ctest 67/67; 8 public vectors; FM3-FM4 filed; no ABI exports or game runs | (1) Integrate portable core; (2) regenerate question index; (3) resolve FM3-FM4 for native integration. |

## Report

Implemented the published portable cSC4FeatureManager owner in `engine/core/src/feature_manager/`.
Commit `61844d96` adds explicit process-shared state, signed object reference lifetime, raw-resource scanning,
ordered feature decisions and pack queries. Commit `1df2f744` adds eight independent public vectors and
the final boundary tests. No extrapolated owner methods, foreign owner-output delegation, C ABI changes,
plugin changes, ABI-map edits or runtime/default promotion.

Every application manager shares a `FeatureProcess` handle. Its RwLock protects getters used on another
thread; foreign registration, resource-open and release calls run without holding a state lock. The core
owns all 32 ascending resource keys, presence bits, byte-line traversal, exact-off decisions, unsigned
feature ordering and static/object teardown. The highest-pack default dispatches through the same object's
overridable pack query. Portable identity tokens describe aliasing and destructor return identity without
claiming native pointer validity after deletion. Terminal Rust handles return None/false as a safety extension.

The service audit found a compatible RZ INI byte-line parser, reused directly. Rule N already existed in
formats, whose dependency on core prevents using it in reverse. It was extracted unchanged into
`core::number_text`; the existing formats API is retained by a re-export, without adding dependencies.
The older network-rules INI helper merges repeated sections and its numeric helper lacks bare-hex Rule N,
so neither is used here.

## Validation

- Final frozen-source gate at `1df2f744`: **qbuild all PASS**, cargo **4340 passed / 0 failed / 0 ignored**,
  clippy **0 diagnostics** with warnings denied, native build **2/2 edges**, full ctest **67/67**.
- Focused quick checks: initial **2620/0** and native **206/206 edges**; final **3820/0**, dependent capi
  compile-only PASS and native **4/4 edges**. No source edits occurred during the final quick or full gate.
  The initial quick noted a tree change because the implementation and lane handoff were committed during it.
- Added **11 owner unit tests**, **1 shared Rule N boundary test** and **8 public vectors** (`fm_01`–`fm_08`).
  Checks include every pack bit and wrapped aliases, exact lookup/release traces, reentrant registration,
  empty/malformed resources, raw invalid bytes, CR/LF, repeated sections/entries, decimal/hex/overflow keys,
  exact-off behavior, shared-state survival, static rescan guards, QI output preservation and identity,
  signed Release boundaries, destructor flags, overridden descending queries and second-thread getters.
- `qbuild fmt` passes the five implementation files (number_text, feature_manager mod/tests, formats exemplar,
  conformance feature_manager). The combined eight-file check reports pre-existing module ordering in the
  shared core/conformance lib.rs registries and brace layout in conformance runner.rs. Registry edits remain
  additive; no mass reformat was made.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`,
  `fc4verify-quick-test.log`, `fc4verify-quick-deps.log`. Read their parsed verdicts, not only process exits.
- Game/shadow: **n/a**. This is a portable-core lane with no C ABI/plugin binding or game request.

## Questions and integration limits

- **FM3**, `docs/specs/questions/feature-manager.md`: resolved-file streaming read failures and partial INI
  enumeration are unpublished. The provider requires readable raw bytes. Unresolved keys remain distinct
  from resolved empty/malformed files; no read failure is guessed into a miss. Native streaming failure
  integration remains missing pending an answer.
- **FM4**, same file: AddRef overflow is unpublished. The code comments and tests identify the literal
  wrapping 32-bit increment choice; signed Release applies to that resulting count.
- Existing FM1/FM2 concern reachability and do not change this owner's published decisions. The lane did not
  inspect their answered archive. The coordinator must regenerate QUESTIONS.md; this lane may only append
  its question file and may not execute tools/.
- App resource registration, cached/borrowed getters, native callback binding, vtable and heap mapping belong
  to later integration. The raw-resource provider supplies the highest-priority already-resolved file and
  releases only the reference it returned. None of these adapters can answer owner feature/pack queries.
- q.py cannot start Bash here (E_ACCESSDENIED); native PowerShell and qbuild work. qbuild fmt requires absolute
  file paths. Git commits succeeded on detached HEAD despite a packed-refs.lock permission warning; no refs
  were switched or manually updated. Supervisor publication remains authoritative.

## Owner checklist (published bodies only)

- [x] Constructor: zero signed count, uninitialized object; no static reset.
- [x] QueryInterface slot 0: same identity for IIDs 1 and 0x0B562673, one AddRef; failure preserves output.
- [x] AddRef slot 1 (shared body, outside 12 unique owner bodies): 32-bit increment/register result; FM4 choice.
- [x] Release slot 2: signed >1 decrement; otherwise Shutdown and deleting destructor flag 1 without decrement.
- [x] Init slot 3: object guard, then static scan; true for every live call.
- [x] Shutdown slot 4: clears only object initialized flag; true.
- [x] IsFeatureEnabled slot 5: unsigned ordered-set membership before/after object Init and Shutdown.
- [x] IsExpansionPackInstalled slot 6: low-five-bit wrapped n-1 shift, including n=0 and u32 extremes.
- [x] GetCurrentExpansionPack slot 7: descending virtual-query default on the same object, or zero.
- [x] Deleting destructor slot 8: returns identity; only bit 0 frees; no implicit Shutdown or static changes.
- [x] Static scan: first guard, register, clear, ascending 32 raw keys, bit before parsing, release opened file.
- [x] Features entry callback: shared Rule N, exact case-insensitive off, ordered insert/erase.
- [x] Static shutdown callback: guarded static reset; no unregister.
- [ ] Unpublished native streaming file failure behavior: FM3; deliberately outside the raw-byte provider contract.

Compiler-generated set startup/teardown are explicitly exempt in §2. App creation/getter/caching and native heap/vtable bindings belong to integration, not additional owner methods. No C ABI, plugin, ABI-map or game changes.

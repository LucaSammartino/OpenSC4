| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-gzframework5: framework core owner closure | Codex / core-gzframework5 | wip: settled contracts implemented and full gate passed; whole-owner closure withheld for spec gaps | Resolve GZF1–GZF11 and finish the module Owner closure table before claiming owners. |

## Implementation

Implemented the settled contracts for all nine owners in
`engine/core/src/gz_framework/` and `engine/core/src/commands/server.rs`.
The [Owner closure table](../../engine/core/src/gz_framework/README.md) maps every
owner/spec section to code, conformance tests, and unresolved questions.

* Framework: QI/output preservation, unusual nondeleting reference count,
  service and tick/ordered registries, mutation protection, initialization/Main,
  quit/abort, scheduling/debug state and exception-report ordering.
* COM: acquired framework/director state, retryable/idempotent Init, distinct
  class/provider fallback lookup, library loading/notification/unload permission,
  and locked dword IDs supplied by an injected provider.
* Command server: registry, lowest-ID case-insensitive lookup, descriptions,
  parameter-set creation, repeated Init, dispatcher/generator registration,
  conversions, synchronous execution and shutdown. Reuses existing parameter
  sets, string generator and message dispatcher. The additive `execute_shared`
  preserves the original mutable API and permits command callback reentry.
* Misc/tool supports: guarded lifecycle and acquired service references, all 35/7
  registrations, and the raw misc duplicate/omitted-notification distinction.
* Buffer: allocator-supplied uncleared row storage, four-byte stride, dimensions,
  type/bpp, nested locks/change counter and format-specific RGB packing.
* Log service: backend registry, filtering, callbacks outside locks, owned-text
  FIFO queues, deferred recursion and depth guard, level settings/sentinels,
  and guarded shutdown of an attached configured host.
* File system: independent lazy data/plugin caches, default Plugins name,
  non-atomic directory setters and stored CD options; path/INI primitives foreign.
* Tagged script interpreter: retained target/extensions/context, vocabulary
  shadowing, byte tokenizer, direct parameter/command/text callbacks, reentry and
  error reset, file/resource execution with resource-return cleanup.

OS operations and hosting application hooks remain traits. No plugin/capi wiring,
C exports, ABI version, generated ABI inventory or coverage-map edits. No game
requests, shadow results or replacement-safety claims.

## Verification

Final stable source: **72a31a00**. Full command:
`python C:/Users/tlori/FreeCity4/.claude/lane-bin/qbuild.py --wt C:/Users/tlori/FreeCity4/.claude/worktrees/core-gzframework5 all`.

**PASS:** cargo summed over all result lines **6464 passed, 0 failed, 0 ignored**;
clippy **0 diagnostics** with warnings denied; CMake **47/47 edges** (107.0 s);
full CTest **69/69** (189.8 s). Native gate 299.1 s, request
`qbuild-20261023-215134-build-ctest`, collected to terminal. No tree-change warning.
Full logs: `build-runs/gate-2-evidence/` and original qbuild/fc4verify logs.

All **GZC01–GZC38** vectors are asserted in 19 grouped tests, plus six edge tests
for dispatch reentry, RGB formats/allocation failure, log levels/lifecycle,
reference release order, explicit open level tables, and NUL/malformed-quote reset.
Owned-file formatting passed (10 files, then the three corrected files).

Earlier evidence is preserved: focused-1 compile failure and correction in
`build-runs/focused-1-diagnostic.txt`; focused-2 19/19 PASS in
`build-runs/focused-2-result.txt`; gate-1 logs in `build-runs/gate-1-evidence/`.
Gate-1 passed 6462 tests, clippy and 69 CTests, but a documentation commit during
its run caused a tree-change warning. Review then found open-spec level/NUL paths
that needed explicit Unsupported results; corrected source was committed and the
full gate above rerun. No duplicate asynchronous checks remain outstanding.

## Questions and remaining owner closure

Appended **GZF10** and **GZF11** to
`docs/specs/questions/gz-framework-core.md`. Only this spec question file was
edited; generated question-index regeneration belongs to the coordinator.
Existing **GZF1–GZF9** remain unresolved. Whole-owner coverage is **not claimed**.

The exact open sections are:

| Sections | Remaining contract |
|---|---|
| §2.1–2.3 | unnamed/native stream/accessor/lifecycle entries, callback kind/order ties, partial unwind and loop policy: GZF1/GZF10/GZF11 |
| §3.1–3.2 | service-running/enumeration signatures, class-index callbacks, failed-library/shutdown ownership and native ID algorithm: GZF2/GZF11 |
| §4.1–4.2 | inherited native identity, formatted entry signatures, bulk/enumeration/null-dispatch and generator replacement contracts: GZF3/GZF11 |
| §5.1–5.3 | support QI/final-release details and all misc/tool command schemas, outputs, wait/drag/error adapters: GZF4/GZF5/GZF10 |
| §6 | buffer QI/color-key defaults/lifetime, remaining storage/conversion/resize/attachment/palette, allocation-failure aftermath and conflicting Unlock descriptions: GZF6/GZF10 |
| §7.1–7.2 | logging Init configuration, full level tables, inventory-only stream/output/accessor entries, collision/ownership/drain edge cases and owner-thread policy: GZF7/GZF10/GZF11 |
| §8.1–8.2 | application path splitting/string lifetime, INI literals, cache invalidation and CD discovery/validation: GZF8/GZF10 |
| §9.1–9.3 | native constructor/IID/default vocabulary/token mutation, numeric/unassigned defaults, final/error callbacks, short reads/NUL/malformed EOF: GZF9 |

Open operations return Unsupported or remain explicitly outside the native API;
injected fixture policies are labeled in code/README and questions. In particular,
non-sentinel log conversions and embedded-NUL scripts do not invent native results.
This lane is **wip** pending those contracts, despite passing implementation gates.

Next: review question answers, replace listed open boundaries with their specified
contracts and tests, then rerun the full gate before claiming owners.

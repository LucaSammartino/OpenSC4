| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gdrivers-director4: graphics COM director native behavior specification | `re-gdrivers-director4` / (RE lane, detached worktree) | in progress; round 2 guard context limit; 65-body / 8068-byte inventory and contracts committed through ff6cf97d; native checklist audit PASS; 65 paper vectors; two small spec corrections pending; cargo/clippy/ctest n/a; game n/a; ABI unchanged | 1. Coordinator review of guarded handoff. 2. Commit pending corrections and finalize report/queue only if continuation is authorized. |

## Handoff

- Goal: reconcile all 8068 coverage-attributed bytes without changing ownership/tools/maps; specify remaining director behavior only. Guard denied further work at context 151k/150k; no restart or additional round initiated.
- Prior milestone: `724a6f1b` contains round-1 director dispatch spec and 31 vectors, but its 85-byte exclusive inventory does not discharge the task.
- Done this round: reused reserved detached worktree; checked clean status and read re-driverdraw4 scope (do not reopen capped driver/RD3-RD5).
- Done `b365703d`: exact current attribution 65 bodies / 8068 bytes, including 63 reach/neighbor bodies / 7983 bytes; corrected 85-byte completion claim; factory construction/pool/array failures and coefficient tables.
- Done: corrected parent spec's 85-byte completion claim; new attributed-helper spec covers factories, constructors, pool/working-array failures, integer coefficient tables; GD32-GD38.
- Done `4cf51c6f`: OpenGL attributed branch contracts (extension forwarding, lighting/material quirks, capture failure/leak behavior, metadata, mode callback, thunks/destruction), GD39-GD50.
- Done `ff6cf97d`: second-factory reusable buffers, save/restore/capture, tile tags, aggregate storage, thunks/cleanup; GD51-GD65 and complete checklist. No owner/map changes or false-edge exclusion.
- Done: complete 65-row / 8068-byte checklist published; no false-edge removal or ownership changes; no whole-driver/family completion credit.
- Audit PASS: ignored `build-runs/audit-director-round2.py` matches all 65 addresses/sizes/rules against `director-attributed.tsv`; 8068 bytes unchanged; GD1-GD65 contiguous. `git diff --check` PASS.
- Pending tracked corrections (NOT committed): inventory spec default template has nine pointer zeros but only eight associated value zeros; parent wording now accurately says reach/neighbor rather than all reachable. Preserve these edits.
- Next exact: review/commit those two pending spec corrections; optionally verify literal OpenGL product-name punctuation via native pointer 0x00B1225C and constructor root citation offsets; then finalize lane report and queue only with coordinator authorization.
- Final attempted native string/root query was DENIED before execution. No REA, game or implementation/test output used. No merge-queue file written this round; existing queue file is stale round 1 and must not be consumed for round 2.
- Inventory snapshot (ignored): `build-runs/director-attributed.tsv`, produced read-only without accessing engine/.
- Primary evidence: main checkout `tools/re_query.py`, `decomp/`, `ghidra/export/windows`, `bin/SimCity4.exe`; never engine/.
- q.py cannot start Bash (`E_ACCESSDENIED`); use bounded PowerShell calls.
- Guard manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-gdrivers-director4-r2-manifest.json`.
- No game requests, ABI proposals, external communication, or subagents.

## Report (2026-10-22, round 1/1)

Published [renderer-gdrivers-director.md](../specs/renderer-gdrivers-director.md). The exact per-body
checklist separates the two exclusive owner bodies from shared dispatch targets and supporting
factories/containers. The contract covers construction, singleton access, registration, service/path
binding, containing-director registration, factory lookup and cleanup, class enumeration, both
interface identities, reference/unload behavior, no-op lifecycle callbacks, path/configuration access
and destruction. [renderer-driver.md](../specs/renderer-driver.md) received only a scope cross-link;
RD3–RD5 and the capped driver body contracts were not reopened.

### Attribution and measured results

- Export/decomp attribution spells the owner `cSC4GDriversCOMDirector`, not the literal task suffix
  `cSC4GDriversCOMDirector8068B`. Exact directly named inventory: constructor `0x00466250`, 79 bytes,
  and ID getter `0x004662A0`, 6 bytes: **2/2 bodies, 85/85 bytes specified**.
- All **17 primary slots** checked against executable table `0x00A87EE8` and native target bodies.
  Sixteen shared targets total **506 bytes**, including the 30-byte folded deleting destructor; do
  not add that destructor twice or award those bytes exclusively to this class.
- All **11 secondary entries** checked against executable table `0x00A87EB8`. Three identity-adjusting
  thunks; eight callbacks use the same true-returning no-op. The shared `PreFrameWorkInit` at
  `0x0087B1FC` is not on this director's table.
- Constructor registration order: `C4554841`, `7ACA35C6`, `BADB6906`, all no-argument factory kind 1.
  Native registry requests 100 buckets and selects 193; unsigned remainders 113,40,179 produce the
  different callback order **7ACA35C6,C4554841,BADB6906**. No driver is constructed by enumeration.
- Unsupported factory-interface requests call **QI → AddRef → Release**. Null factory results are
  dereferenced without a Boolean failure path. QI/missing-class failure preserves an output sentinel
  unless a delegated child/factory changed it.
- InitializeCOM copies a source string interface by length, preserving embedded zeros; children get
  the original service/path arguments and their results are ignored. Null service returns false before
  changing state; nonnull service plus null path dereferences invalid memory after storing handles.
- Release saturates at zero without destruction. CanUnloadNow returns true at nonzero own reference
  count and skips children; at zero it asks children. Singleton exit performs base cleanup directly,
  without freeing static object storage or resetting its construction guard.
- **GD1–GD31** are independent discriminating paper vectors, not executed implementation/game tests.
  Supporting shared-body fixtures are distinguished from the director's default childless registry.

### Evidence and validation

Used only the main checkout's native export, decompilation and working executable data/instructions.
Numeric constants, operand widths, comparison signedness, stack arguments and order were checked with
`re_query.py asm --bytes`; table data were read from the local PE. Export queries came first. REA was
not needed. q.py could not start Bash (`E_ACCESSDENIED`), so bounded PowerShell calls were used.

The ignored one-off `build-runs/audit-director-doc.py` confirms the direct inventory, all 17 primary
targets/sizes, 11 secondary entries, the 506-byte shared total and 31 contiguous vector IDs against
native metadata/PE data. Audit **PASS**; `git diff --check` **PASS**. This is documentation consistency
validation, not runtime behavior evidence. Cargo, clippy and ctest: **n/a**, docs-only. Game runs:
**n/a**; no game requests or shadow/replacement claims.

Milestones: `88bcbb84` scope/handoff; `b7a7bc26` substantive registration/lifecycle/failure contract;
`751ab8a3` full dispatch/checklists/order and 31 vectors. Final review refines attribution wording,
adds the existing-spec cross-link and validates native inventory consistency.

### Questions, ABI and next steps

Questions filed/answered: **none**. No question index change was required. No ABI exports, ABI tables,
ABI proposals, engine, tool, map, game or external files were changed or committed. Helper scripts
remain ignored under this worktree's build-runs. Worktree stayed detached; no branch/ref switch,
merge, push, restart or subagent was used.

Ready for a docs train. Coordinator should reconcile the brief's **8068B** figure before any coverage
bookkeeping: the native inventory supports 85 exclusive bytes and separately documented shared
dispatch, not an 8,068-byte director or whole-DX7-family completion claim. No further body in this
director's actual attributed inventory or published dispatch surface remains unspecified here.

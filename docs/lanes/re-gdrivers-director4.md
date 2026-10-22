| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gdrivers-director4: graphics COM director native behavior specification | `re-gdrivers-director4` / (RE lane, detached worktree) | in progress, round 2; reconciling coverage-attributed 8068 B; prior exclusive-body completion claim superseded; cargo/clippy/ctest n/a; game n/a; ABI unchanged | 1. Recover exact coverage body set. 2. Trace remaining helpers or prove false reference edges. 3. Publish per-body checklist and vectors. |

## Handoff

- Goal: reconcile all 8068 coverage-attributed bytes without changing ownership/tools/maps; specify remaining director behavior only.
- Prior milestone: `724a6f1b` contains round-1 director dispatch spec and 31 vectors, but its 85-byte exclusive inventory does not discharge the task.
- Done this round: reused reserved detached worktree; checked clean status and read re-driverdraw4 scope (do not reopen capped driver/RD3-RD5).
- Done: recovered exact current attribution: 65 bodies / 8068 bytes, including 63 reachable/neighbor bodies / 7983 bytes. Factory edges are actual instructions; no ownership change justified.
- Done: corrected parent spec's 85-byte completion claim; new attributed-helper spec covers factories, constructors, pool/working-array failures, integer coefficient tables; GD32-GD38.
- Done: OpenGL attributed branch contracts added (extension forwarding, lighting/material quirks, capture failure/leak behavior, metadata, mode callback, thunks/destruction), GD39-GD50.
- Done: second-factory reusable buffer lifecycle, copy/save/restore/capture, tile-tag arithmetic, aggregate storage, thunks and destruction order specified; GD51-GD65 added.
- Done: complete 65-row / 8068-byte checklist published; no false-edge removal or ownership changes; no whole-driver/family completion credit.
- Next: audit checklist against saved native attribution and vector IDs, diff-check, correct any discrepancies; finalize lane report and queue.
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

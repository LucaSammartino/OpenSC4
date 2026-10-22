| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-gdrivers-director4: specified director core | `core-gdrivers-director4` / (engine lane; detached worktree, supervisor publishes reserved branch) | ready; code 5e0b764d; cargo 4931/0; clippy 0 diagnostics; ctest 68/68; 80 conformance scenarios; ABI unchanged; no game/shadow/replacement claim; questions GD1, GI1, GI2 | 1. Review documented specification boundaries. 2. Coordinator merge. |

## Report

Implemented the bounded cSC4GDriversCOMDirector inventory in
`engine/core/src/gdrivers_director/`, with additive core/conformance wiring only.
The [per-body proof checklist](../../engine/core/src/gdrivers_director/README.md)
reconciles exactly **65 bodies / 8,068 bytes**, plus the separate primary/secondary
director dispatch checklist. No C ABI, plugin, CMake, runtime/default, map or
replacement wiring changed. No completion of nSGLDX7, nRZSimGL, cGZBufferSimGLCache,
RD3–RD5, or a whole DX7 family is claimed.

Implemented director construction/identity, path copying, service/framework binding,
child flattening and ordering, registration/lookup/enumeration, bucket/collision order,
reference and unload quirks, lifecycle no-ops, singleton access/exit and destruction.
The attributed helpers include the first two factory products, identity thunks,
extension/light/material forwarding, front/software capture, mode callbacks,
coefficient tables, working arrays, block pool, reusable buffers, save/restore/tile
tags, shallow A/B records and storage, and both product cleanup/finalizer sequences.

Host operations are primitive foreign memory, COM, services and GL/WGL operations;
the attributed algorithms execute in Rust. The third product, primary driver QI/refcount
bodies, mode/device delegates, optimized endpoints and general allocator internals
remain foreign. Native unchecked failures use explicit Rust faults after preserving
preceding effects, not invented Boolean failures. Arithmetic/addresses wrap at 32 bits;
low-byte Booleans, partial initialization, stale fields, shallow pointees and cleanup
order remain explicit. Unknown values use partial records or known-bit masks.

## Independent verification

- Eight JSON files encode **GD1–GD65 plus 15 supplementary public scenarios**.
  Expectations were fixed from the two specifications before execution. No old
  renderer or implementation supplied outputs. The fixture poisons allocations and
  records primitive calls, writes and frees; a negative control rejects changed expectations.
- Coverage includes sentinels, callbacks/child order, interface adjustments, reference
  balance, duplicate/collision registration, destruction instead of an invented
  unregister API, allocation faults, signed/wrapping arithmetic, serial reuse, empty
  copies, negative pitches, tile-edge clears, self-aliasing insertion and partial defaults.
- Stable quick gate: **4,251 passed / 0 failed**, dependents compiled; build **4/4 edges**.
- Final stable `qbuild all` at code **5e0b764d**: **4,931 passed / 0 failed / 0 ignored**;
  clippy **exit 0 / 0 diagnostics**; native build **4/4 edges**; ctest **68/68**, zero
  failures. Native verification **435.6 s** (build **147.7 s**, ctest **283.6 s**).
  Console verdict: **qbuild: PASS**.
- Logs: `build-runs/qbuild-test.log`, `build-runs/qbuild-clippy.log`,
  `build-runs/fc4verify-build.log`, `build-runs/fc4verify-ctest.log`.
- Absolute-path `qbuild fmt` passes all **six new Rust files**. Checking all nine
  touched Rust files reports **211 diff lines**, beginning with existing module
  ordering in shared wiring. Lists were kept additive as instructed; unrelated
  module ordering was not mass reformatted. Relative-path fmt resolves incorrectly
  here; use absolute paths. The canonicalization warning is not a format difference.
- Initial baseline attempt: cargo **4,922/0**, clippy **0 diagnostics**, build
  **208/208**, ctest **67/68**. This was invalid as a stable baseline: its later doctest
  saw partially added modules during implementation. No redundant baseline was run.
  The first full implementation gate passed cargo/ctest but caught two Rust-1.82-only
  `repeat_n` uses; the final gate verifies the corrected Rust-1.75-compatible code.
- No game request was made. Game verdicts, reference warnings, shadows and replacement
  are **n/a for this pure-core lane**. No runtime equivalence is claimed.

## Questions and boundaries

- **GD1**, [director questions](../specs/questions/renderer-gdrivers-director.md): full
  registry size table and rehash chain order are absent. The specified 193-bucket
  regime is exact; growth beyond it raises a specification-boundary fault. Provisional
  GDQ1 was normalized to the documented two-letter prefix; this question is distinct
  from the parent spec's GD1 paper vector.
- **GI1**, [inventory questions](../specs/questions/renderer-gdrivers-director-inventory.md):
  matrix extent and aggregate placement among constructor subobjects are unspecified.
  Initial matrix words are zero without a guessed shape; aggregate construction uses
  the literal pool-before-aggregate-before-working-arrays reading. Only the known-clear
  state bit is promised.
- **GI2**, same file: general pool descriptor/set allocation policy and virtual-release
  arguments are unspecified. Logical descriptor order and heap/virtual ownership are
  preserved without claiming native general allocator internals.

These boundaries are documented in code and the proof checklist. Review them before
expanding the API or claiming equivalence beyond supplied contracts. The coordinator
regenerates the question index; this lane edited only the two allowed question files.

## Commits and next steps

- **1483bb68**: director and attributed helper algorithms.
- **a6144ebe**: independent conformance and per-body proof checklist.
- **5e0b764d**: MSRV correction, explicit portable cleanup, question prefix.

Coordinator: review GD1/GI1/GI2 and merge the pure-core change. Runtime adapters,
game comparisons and other driver owners require separate work. The supervisor
publishes this detached worktree's actual HEAD to the reserved lane branch.

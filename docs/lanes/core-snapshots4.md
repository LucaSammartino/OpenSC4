| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-snapshots4 (portable snapshot album and taker) | `core-snapshots4` / `.claude/worktrees/core-snapshots4` (engine lane; detached HEAD) | Ready: production code through 66f778c1; final gate at 039f2b1b: cargo 5003/0, clippy clean, CMake 4/4, ctest 68/68; 16 snapshot fixtures passed. SN3-SN9 filed. No C ABI changes, game runs, native-exact/replaced or full-owner credit. | (1) Resolve SN1-SN9 before extending native compatibility; (2) supply native adapters only after missing contracts are published. |

## Report

Implemented `engine/core/src/snapshots/` from snapshots.md sections 1-4/6.
This is a bounded portable implementation, ready for review and integration;
it does **not** earn complete native owner, native-exact, shadow or replaced credit.
No game requests were made. No Documents, installed game files or user pictures were accessed.

### Production behavior

- `SnapshotAlbum` owns the root, album/picture lists, both selected indices,
  description edit state and decoded information. It implements album creation,
  filtering/order, city/index selection, initial/current picture selection, count
  and no-image state, description saving/removal, next/previous, picture/companion
  deletion, album file deletion, imports and shutdown.
- Imports keep an existing same-folder name, otherwise allocate the documented
  city/date base name, copy bytes before source removal when requested, write
  metadata/description companions and select the imported picture directly.
  An unrelated picture's malformed metadata cannot replace that selection.
- Information writing owns all ten LF-ended fields, signed 64-bit funds,
  unsigned comma-separated view values and XOR 0x7B. Reading owns deobfuscation,
  marker checking and field splitting. Caption inputs remain structured pending
  SN2; no native caption output or layout is borrowed.
- `SnapshotTaker` owns supplied sizes, size index, bounded rectangle, drag anchor,
  explicit digit percentage, key/modifier decisions, capture counter and close/
  restore state. The UI entry point hides before posting counter 0 and raising
  the Lua event. Three re-posts precede capture on the fourth delivery.
- The capture workflow selects whole-view versus rectangle, requests opaque
  alpha for 16/32-bit images, creates/selects the city album, writes PNG bytes,
  removes partial output after a failed write, adds metadata and opens the modal
  album. Modal completion restores the prior pause and closes/restores the frame.
- `MemoryFs`, validated virtual `VPath`, and `MemoryUi` are usable in-memory
  primitives. Clock/date, raw capture/surface/codec and byte IO boundaries are
  typed. There is no OS filesystem adapter, native snapshot-owner callback,
  toolkit inference, new dependency or C ABI change. Only the module declaration
  was appended to the shared `core/src/lib.rs` list.

### Owner and byte inventory

The audit preceded implementation and is retained in
`engine/core/src/snapshots/OWNER_INVENTORY.md`. It inventories every named method
in the relevant spec sections and separates portable algorithms from missing
contracts. All other read ABI sources had no matching owner rows.

- Taker: 151 vtable slots / 604 table bytes; nine owner-target rows, 142 other
  toolkit/interface targets. Published object allocation: 0x10C; percentage field:
  +0xE8. Paint, native lifecycle/destructor, inherited toolkit surface, exact
  help composition and uncertain return channels remain unbound.
- Album: six vtable slots / 24 table bytes; two owner-target rows and four
  interface/toolkit targets. Native object size/layout is unpublished in the
  audited sources. Native script/control/message binding and destructor behavior
  remain unbound.
- The exact 157-row extraction is retained locally in
  `build-runs/snapshot-abi-inventory.tsv`; no shared ABI tables were edited.
- File inventory: empty creation-time `album.ini`; recognized png/jpg/jpeg/jp2/
  gif/bmp extensions; stem `.info` and `.txt` companions; XOR-obfuscated ten-field
  information; raw description bytes with empty-description removal. Pixel
  masks and PNG encoding remain image/codec primitives, not guessed wire formats.

### Measured validation

Final stable `qbuild all`, gate HEAD `039f2b1b`, source through `66f778c1`:

- Workspace tests: **5003 passed, 0 failed, 0 ignored**.
- Snapshot fixtures: **16 passed**, independently counted in the workspace log.
- Clippy with warnings denied: **exit 0, diagnostics 0**.
- CMake: **4/4 edges**, 158.8 s, sccache Rust 0 hit / 3 miss.
- Full ctest: **68/68**, 352.3 s (ctest reports 352.13 s real time).
- `fc4verify`: **PASS**, 513.6 s; `qbuild`: **PASS**. No source changes or
  tree-change warning during this final gate. Working tree was clean afterward.

Final logs are `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
`fc4verify-build.log` and `fc4verify-ctest.log`.

The first six expected fixtures were committed in `757008ec` before production
implementation (`d58b77cb`). Subsequent fixtures cover copy/move and failed-save
ordering, partial-file cleanup, alpha-depth selection, pause restoration,
clock/date inputs, path scope, frame reuse/drag, same-file dirty descriptions and
UI hide/post/Lua effect order. The last effect-order correction is `66f778c1`.

Earlier results and limitations are preserved:

- Initial `all` attempt: 4987 Rust tests passed; clippy exited 101; CMake failed
  at 44/212 and ctest was skipped. The tree changed during later phases, so this
  attempt cannot attribute baseline failures reliably. Logs are preserved in
  `build-runs/baseline/`; the final stable pass above supersedes it for this code.
- Iterative quick: **PASS**, core 2965/0 (including the original six snapshot
  fixtures), dependent crates compiled, CMake 170/170; 1427.5 s total. Source
  changes overlapped that run. Its logs are preserved in
  `build-runs/quick-iteration/`; final `all` ran all 16 fixtures on frozen code.
- Fmt check passes for the six new Rust files. Checking touched shared `lib.rs`
  returns exit 1 / 166 diff lines proposing module reordering and existing export
  reflow, including relocation of the additive new module entry. Kept the shared
  list additive at its end as instructed; no unrelated formatting was applied.

### Questions, residuals and next steps

Appended **SN3-SN9** to `docs/specs/questions/snapshots.md`; SN1-SN2 were already
open. No other spec files were edited.

- SN3: sanitizer/encoding/48-byte rules, collision suffix/search and region-only
  naming. Already-valid UTF-8 names use a character-boundary prefix; unsupported
  sanitization, an occupied base PNG name and region-only naming return explicit
  contract gaps. Repeated same-city/date captures therefore need the collision
  contract before they can allocate a second name.
- SN4: collation, picture order, case, initial selection, browse endpoints,
  post-delete choice and album-switch saving. Literal provisional choices are
  marked in code: bytewise order, first initial picture, checked indices,
  clamped endpoints, successor-or-last deletion and save before switching.
- SN5: text encoding and malformed info/description behavior. UTF-8 writer text
  refuses embedded LF; reader preserves raw fields and reports incomplete marked
  records as a gap. Caption layout remains SN2; ninth field meaning remains SN1.
- SN6: size/default/oversize, positioning/drag and pending-input details. Supplied
  fitting sizes are supported; oversized sizes return a gap. No initial native
  percentage value is fabricated.
- SN7: missing named native lifecycle/paint/destructor, resources, controls,
  messages, slot semantics and return data. These prevent full-owner credit.
- SN8: pixel masks/crop conventions and failure/modal behavior. Surface/codec
  work remains a primitive boundary; failure close/restore is explicitly marked.
- SN9: IO failure/rollback/overwrite, nested folders and path comparisons.
  Operations stop on errors; nested album directories return a gap before
  deletion. No recursive deletion or undocumented transaction is invented.

Next work is spec clarification followed by the missing portable contracts and,
separately, typed native adapters. This round makes no claim for those residuals.
The supervisor must publish the detached commits to the reserved lane branch;
this lane did not switch branches, merge, rebase, push or modify shared refs.

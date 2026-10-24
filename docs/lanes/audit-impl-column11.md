| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| audit-impl-column11: implemented coverage attribution | Codex; audit-impl-column11 (detached HEAD) | ready; doccheck passed; +53,555 proposed bytes | Coordinator review and apply high-confidence proposals |

## Report

The audit proposes 26 high-confidence mappings and records 96 partial-owner exclusions in
[proposed-core-column.tsv](../specs/attribution/proposed-core-column.tsv). Evidence is the
clean-room module documentation, owner closure tables, referenced specifications and test
sources. No engine source, ABI exports or tools changes are committed. No original program,
export or decompilation was inspected directly; the authorized coverage command performed
its own attribution calculation.

All **110 distinct existing core paths exist**. The full literal inventory contains **45
core directories and two formats directories**, plus **25 core and 11 formats top-level
Rust files** without exact column-5 references. The TSV lists every item and distinguishes
production owners, seven sibling-module test directories, fixtures/utilities and shared
helpers. Existing directory references can cover sibling source files without naming them
literally; this audit checks existing path existence, not every existing owner's closure.

Supported additions cover the graphics director's 65-body inventory, three binding/material
owners and three literal getter/helper rows, six editor controls, path/directory owners and
helpers, regional city and the seven-method overlay vertex helper. The directory owner
actually lives in rz_files and is re-exported by paths. Existing pattern order is preserved.
Broad catch-all patterns receive no credit from isolated helpers or partial owners.

Important exclusions include View3D integration, audio, unfinished widgets, web transport/cache,
renderer boundaries, lot editor/map/place-lot gaps, the explicitly partial persistence inventory,
vehicles/aircraft and texture factory. Its test
`restore_distinct_records_withholds_unknown_hash_order` confirms that ordinary multi-record
restore reports `RestoreTableOrder` and emits no uploads. Testing an unsupported result does
not close the owner algorithm. Feature-manager's complete-byte provider likewise cannot
execute the streaming failures now specified in section 4.1.1.

### Measured results

Both runs used `python tools/coverage.py --owners ... --out ...`. Only the 26 high values
were temporarily applied to this worktree's map with apply_patch, then restored. The final
map diff is empty. A Windows stdout encoding error initially damaged comment characters
during restoration; an explicit UTF-8 read and ASCII JSON patch transport corrected it,
and the final empty diff confirms complete restoration. Neither measurement changed attribution
inputs or first-match order. Reports and per-function evidence remain in build-runs/.

| Metric | Baseline | Proposal | Change |
|---|---:|---:|---:|
| Implemented bytes | 2,907,143 | 2,960,698 | +53,555 |
| Implemented functions | 12,593 | 12,879 | +286 |
| Share of game bytes | 52.1% | 53.0% | +0.96 percentage points before rounding |
| Game denominator | 5,584,842 bytes / 29,536 functions | unchanged | 0 |

Byte contributions: director 8,068; binding/material/getter helpers 517; six editor controls
13,488; paths/directories 6,762; regional city 23,713; overlay vertex helper 1,007.
`fn:ResolvePath` matches zero currently attributed functions and adds zero bytes.
Specified/verified/replaced remain 78.0%/27.0%/19.1%. This is accounting evidence, not new
native verification, shadow evidence or a game verdict.

Validation: all 122 exact patterns, uniqueness and proposed paths passed; doccheck passed.
Cargo/clippy/ctest and game runs are n/a for this documentation-only audit. No new questions
or ABI-row proposals. Git commits succeeded despite a nonfatal shared packed-refs.lock
permission diagnostic; no shared refs/configuration were changed.

## Handoff

- Goal: propose evidence-backed column-5 attribution without committing coverage-map changes.
- Done: initial lane commit 5cfe25d3b; enumerated all unreferenced directories/files and checked existing core paths (none missing).
- Done: inspected module docs, closure tables, spec references and test inventories; drafted proposed-core-column.tsv with separate high/partial sections.
- Measured baseline: 2,907,143 implemented bytes / 5,584,842 game bytes (52.1%); 12,593 implemented functions / 29,536.
- Key exclusion: texture factory multi-record restore explicitly returns RestoreTableOrder without uploads; unit vectors do not close this owner.
- Next: coordinator reviews and applies approved high rows only; partial rows retain '-'.
- Final measurement: +53,555 bytes / +286 functions; map restored and doccheck passed.
- Evidence: build-runs/coverage-baseline.md, coverage-proposed.md, owners-baseline.tsv, owners-proposed.tsv.
- No outstanding game labels, questions or ABI changes.
- Constraints: clean-room engine/docs only; no decomp; no tools changes committed.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-audit-impl-column11-r1-manifest.json.

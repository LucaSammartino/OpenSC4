| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-geometry-utilities-contracts4: public geometry delta | Codex, `core-geometry-utilities-contracts4` | ready: stable all PASS, cargo5511/0/0, clippy0 diagnostics, CTest68/68; bounded portable scope only | Coordinator may land existing delta; GU1/2/3/5/6/GU7 and exact vector limits remain |

## Report

Implemented the newly public bounded geometry delta in core only. No C ABI,
plugin, defaults, maps, game requests, geometry owner-body native measurements
or foreign algorithm changes. **Ready within the bounded portable scope**:
one replacement stable canonical all completed successfully on unchanged committed
engine source. This final round changed only this report; no algorithm expansion.

Commits: a41cb7c2 records the independent ledger before comparisons;
1e6c4bf7 adds the implementation; bd03cb5f reconciles operation stages,
public DIFFERS expectations and the exact 37-body supported/partial checklist.
The reserved detached worktree was reused; final round began at clean383385db.
No refs/config/guard flags were changed and no agents were launched.

### Changes and domains

- `raw_texture`: primitive font handles/refcalls/modes/metrics/draw, surface
  factory/create/access/bytes/line and descriptor/upload calls. Core counts
  bytes to NUL, owns wrapping signed integer layout, bounded terminating power
  loops and nearest binary32 output conversion. It restores mode then writes
  outputs then releases nonnull surface then font on every defined post-font
  failure. Mode3 spans upload; false raw draw/line results are ignored. Texture
  is borrowed without AddRef/Release. Null/uninitialized/nonterminating caller
  domains are typed exclusions, not invented native graceful failures.
- Plain upload owns unsigned code dispatch, supported format/type, signed
  pitch/BPP truncation, backed rect offset/dimensions, actual upload result,
  end-access and service cleanup. Unset unsupported descriptors are not used.
  Plain auxiliary fields are unspecified (GU7); compressed auxiliary is [1,1].
- Compressed upload owns row-major DXT3 alpha nibble packing, endpoints/indices,
  positive aligned backed pitch stepping, descriptor and temporary-free before
  end-access, including false upload. Early failed checks/access do not unlock.
- Signed clipping owns n/m/max/count and backing independently. Initial positive-n
  copies precede m/max logic; each edge has distinct capacity40 scratch, then
  signed min(emitted,max) and positive-count copy-back. Tails survive. Crossing
  calls precede capacity gates, successful scratch is reused and uninitialized
  scratch reads return typed Unsupported at that operation. Portable backing
  checks exclude native unsafe overflow. Legacy Vec APIs are qualified adapters.
- `pc24` is explicit nearest-even/masked, finite normal/zero arithmetic, with
  typed boundaries for extended exponent, subnormal and unsupported inputs.
  Length and plane associations are per-function. Double lighting constants
  use integer dyadic rounding, magic colour store/bit/low-byte subtraction is
  owned, and mesh u16 slot/count wrapping and u32 inline closure are owned.
  Reset preserves dirty/bounds; AddVertex does not dirty or close primitives.
  Foreign bounds/container/destructor behavior remains unproved and unchanged.
- Legacy normal-range numeric adapters now follow the published per-function
  associations and branch/output gates, including quiet-NaN/signed-zero overlap
  selection, nullable-minimum gating and required-maximum checked exclusion.
  They are not ambient x87, PC64, extended-exponent or native-trig models.

### Measured validation

- No pre-edit baseline: task-specific instruction explicitly forbade it. Historical
  20 tests / 4309 Rust / 67 CTest are provenance, not a measurement for this source.
- First focused quick: 3410 passed / 2 failed, stale assertions for plane
  association and surface-failure output policy. Corrected against public DIFFERS.
- Second focused quick **terminal PASS**: Rust **3467 passed / 0 failed / 0 ignored**
  over **5 summaries** (3414+21+13+19+0); all dependent crates compile-only;
  native **4/4 edges**, wrapper exit0, 322.9s. It explicitly reported tree changes
  during development, so this is focused evidence, not the canonical stable gate.
- qbuild fmt: **exit0, 9 files**, no format diffs (one external canonicalization
  warning). Relative file arguments first failed to resolve; absolute paths passed.
- Historical canonical all started on committed source
  **bd03cb5f7edaf2abaeca7d3bc1cfb0a9216ab4c1**, engine tree
  **4f6381b35ed54cd78fc8280308cbc9e5f1fd9b2d**. Historical Rust5511/0/0 over26
  summaries and Clippy finished52.22s are preserved. Native4/4 linked edges lack
  a terminal return receipt; later CTest20/68 lacks a final footer. Its canonical
  terminal/native RC remain UNKNOWN, not FAIL or PASS. Final collector artifacts
  `geometry-canonical-terminal.log` and `geometry-source-evidence.json` were absent.
- Session9399 recovery attempted exactly once in this round: `Unknown process id`.
  Archived all43 existing build-runs files (686683 bytes), including raw focused
  and canonical logs, q receipts, source helpers/records and ROOT journals,
  into `build-runs/geometry-final-r2/historical/` before overwriting fixed logs.
  `historical-index.json` records verified per-file byte counts and SHA256.
- **Final stable canonical all PASS**, 2026-10-23 06:43:32.510145 to
  06:46:38.190914 UTC, same tool session73592 collected to actual terminal0.
  Exact command: `python C:/Users/tlori/FreeCity4/.claude/lane-bin/qbuild.py
  --wt C:/Users/tlori/FreeCity4/.claude/worktrees/core-geometry-utilities-contracts4 all`.
  PID27380 and cwd/argv/timestamps/terminalRC are recorded in
  `build-runs/geometry-final-r2/operation.json`; terminal output in `terminal.log`.
- Every workspace Rust summary independently aggregated: **26 summaries,
  5511 passed / 0 failed / 0 ignored**, test exit0. **Clippy exit0,
  0 diagnostics**, finished0.24s (external path-canonicalization warning retained).
  Native build **PASS2/2 edges**, 2.3s; **CTest68/68**, 0 failed, full final
  footer136.8s. Native wrapper `fc4verify: PASS`141.7s returned successfully
  through qbuild, whose actual terminalRC is0. No separate raw native child-RC
  receipt was emitted; linked edges alone were never used to qualify this run.
- Before/after source HEAD **a450261ab662ff192f891dd59edaa6fbb99ad61c**;
  engine tree **4f6381b35ed54cd78fc8280308cbc9e5f1fd9b2d**, clean both times.
  All **2593 tracked engine files** have matching before/after SHA256 records;
  aggregate **92802acfe11914a80bd40d2c3c7c5bf98b42ac348867ae6d0f23e66859020a5c**.
  Records: `source-before.json`, `source-after.json`; independent summary and
  immutable final log copies/hashes: `summary.json`, `completed/`, all under
  `build-runs/geometry-final-r2/`. Subsequent report-only commits do not change
  the measured engine; content/tree checks are independent of report HEAD.
- Old expectations changed only for documented public DIFFERS: plane association,
  tied overlap operand, unsupported failed-crossing scratch, common text-failure
  outputs, negative width's terminating integer dimension, and Reset's retained bounds.
  Existing production plain/DXT3 payload tests remain and pass.

### Historical interruption and final authority

At **2026-10-23 06:16:39 UTC**, the old client was waiting (created
06:10:06.204403 UTC): **PID18344**, parent **30268**, argv
`C:/Users/tlori/.cargo/bin/sccache.EXE --show-stats --stats-format json`, cwd
`C:/Users/tlori/FreeCity4/.claude/worktrees/core-geometry-utilities-contracts4`.
Its established socket is **127.0.0.1:59966 → 127.0.0.1:4463**.
Parent30268 is the lane's `fc4verify.py --wt <WT> build ctest`, created
06:08:18.364310 UTC; parent31440 is this lane's `qbuild.py --wt <WT> all`,
created06:05:33.375984 UTC; its parent is the canonical collector PID38756.
Full exact argv/cwd/creation/parent/port proof is in
`build-runs/geometry-process-proof.json`, now also archived. This is historical
evidence, not fresh liveness. ROOT's copied journal records only optional client18344
exit15 at07:21:46.530467 local, with the parent/daemon alive immediately afterward.
Later parent/backend disappearance causes remain UNKNOWN. The copied07:35:02
scoped census reports no own compiler refs/errors and six clear output probes;
daemon14096 on port4463 is a persistent cache, not an owned compiler.

Final admission and R2 manifest authorize exactly one replacement all after
missing-receipt recovery and archive. The replacement completed; no baseline,
quick, focused repeat, process interruption, cache RPC/restart, environment/config
change, agent, fourth round, merge/rebase/push or shared-state mutation was performed.
Only detached-HEAD commits were made; Git emitted a packed-refs.lock permission
warning during the handoff commit, which still completed normally.

### Questions and next steps

GU7 appended to `docs/specs/questions/geometry-utilities.md`: plain helper auxiliary
descriptor fields are not specified. Only this allowed question file was edited;
ROOT can regenerate the question index. GU4 is publicly answered; GU1/2/3/5/6
residuals remain open. README enumerates all37 bodies without 37/37, 7509-byte,
native, full-owner, runtime or default credit.

Coordinator may land this bounded delta using the completed stable gate above.
No new questions in this final round. N3 PC64 and the exact M2 sentinel17 fixture
remain UNRUN; M1 intermediate words remain UNASSERTED. AmbientPC64, extended
exponent/transcendentals, unchecked/uninitialized domains and foreign ownership,
cleanup and clipping-tail qualifications remain as published. No game runs or
shadow claims (n/a), whole37/7509 closure, owner/default/native geometry credit.
Current authority: `C:/Users/tlori/FreeCity4/build-runs/run4-core-geometry-utilities-contracts4-r2-manifest.json`.

## Independent public vector ledger (before output comparison)
Expected values are copied from public §10.1; these are proposed, not native measurements.

| ID | Independent expected discriminator | Initial state |
|---|---|---|
| N1 | squared length 4B800001 | UNRUN |
| N2 | plane w BF800000 | UNRUN |
| B1 | tied extrema choose second signed zero | UNRUN |
| B2 | unordered extrema choose second; point NaN outside | UNRUN |
| B3 | touching false untouched; null minimum gates maximum | UNRUN |
| C1 | 41 initial copies, count 40, tail (40,0) | UNRUN |
| C2 | m<=0 preserves count 41 and all initial copies for any max | UNRUN |
| C3 | count min(40,max), nonpositive no copy-back | UNRUN |
| C4 | max=0 failed crossings do not read scratch | UNRUN |
| C5 | edge n<=0 count 0; polygon n=-1,m=0 count -1 | UNRUN |
| M1 | alpha 65537 converts to byte 128 | UNRUN |
| H1 | parallel off-plane false writes zero; box false untouched | UNRUN |
| H2 | null ray output chooses different algorithm | UNRUN |
| H3 | unordered triangle accepts, unordered hit | UNRUN |
| H4 | unordered segment parameter false untouched | UNRUN |
| V1 | unordered scale leaves input words | UNRUN |
| N3 | PC64 resident epsilon comparison differs from store | UNRUN / outside bounded PC24 work |
| M2 | u16 vertex wrap; no implicit closure | UNRUN |
| M3 | Reset preserves dirty; Begin appends without closure | UNRUN |
| T1 | GS absent: false; restore, size 32x16, rect 32,13,2,1, font release | UNRUN |
| T2 | upload false: mode3 through upload; common epilogue | UNRUN |
| U1 | base+36, 2x2, stride8; end before cleanup on false | UNRUN |
| U2 | AA*8,0000,FFFF,55*4; free before end on false | UNRUN |
| N4 | squared 4B800001, root 45800000 | UNRUN |
| M4 | channel1=255, negative-lit red=155 | UNRUN |
| M5 | first65535, final23, closure FFFF0018 | UNRUN |

## Portable observations after comparison

These are focused Rust observations, not native runs or whole-body proofs.
The initial ledger above is preserved verbatim. 25 rows have observations;
the qualifications below limit what was actually exercised.

| ID | Focused observation |
|---|---|
| N1 | PASS, 4B800001 |
| N2 | PASS, BF800000 |
| B1 | PASS, second signed-zero minima/maxima |
| B2 | PASS, second unordered operand; NaN point outside |
| B3 | PASS, touching untouched and null-minimum gates maximum |
| C1 | PASS, 41 copied slots, count40, tail40 retained |
| C2 | PASS, m=-1/0 across max=-1/0/1/100 |
| C3 | PASS, signed counts -1/0/1/40 and unchanged copied tails |
| C4 | PASS, max0 avoids failed scratch read; max1 typed Unsupported |
| C5 | PASS, edge count0 vs polygon countn for n=-1/0 |
| M1 | PASS final byte128; intermediate stored words not separately asserted |
| H1 | PASS, plane false zero vs box false untouched |
| H2 | PASS, hit/null selects distinct paths; parallel untouched |
| H3 | PASS, quiet-NaN triangle accepts with unordered coordinates |
| H4 | PASS, unordered segment parameter false untouched |
| V1 | PASS, both original words retained |
| N3 | UNRUN, PC64 resident epsilon outside bounded model |
| M2 | Qualified PASS: u16 wrap, slot0 reuse and dirty/count preservation; count sentinel65535 rather than published17; exact sentinel17 fixture UNRUN |
| M3 | PASS local dirty/reset and repeated Begin observations; foreign bounds/resize behavior unproved |
| T1 | PASS GS/factory/create failures, refs and common outputs; false factory retains a releasable surface |
| T2 | PASS mode3 upload, ignored draw/line false, independent clear/border gates and end-access order |
| U1 | PASS B+36, 2x2, stride8, false upload cleanup; also backed negative pitch -33/4=-8 |
| U2 | PASS exact AB-alpha block/descriptor/free-before-end; padded multiblock row order also passes |
| N4 | PASS squared4B800001/root45800000 |
| M4 | PASS channel1=255, negative-lit red155 |
| M5 | PASS first65535/final23/FFFF0018 closure |

No new game labels; ABI exports unchanged. This report does not promote
GU1/2/3/5/6/GU7 residuals or the historical partial gate into completion evidence.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-nettool-close7 | Codex / glue-nettool-close7 | ready; gate PASS; clean 2032; nettool 8/14 exact; remaining paths default-off | Land measured work; resolve NT35/NT36 before promoting remaining paths. |

## Report

The 2032 turbo run is clean through shutdown, with foreign_exceptions=0 and ref_warnings=0.
Counted nettool drag/status/cost lines are **8/14 exact**, with zero resyncs. The 14/14 goal
is not achieved. Replacement remains default-off, and shadow comparison is opt-in.
This report gives no whole-owner, piece/model or live placement coverage credit.

### Changes

- Reconstructed inherited exact6 content at91fc87d82 after the initial octopus recorded
  parents without their content. Implementation134795b68; final source f2ed3a386.
- Added NT32 retained powerline pricing/release-reset tests and NT34 nonflat A/B/C
  network/terrain solve, grading and refresh boundary tests. These are controlled
  fixtures, not captured native city constraints.
- Fixed network-geometry §9.1/NG1 vertex tie-breaking: SGI193 bucket/chain iteration,
  resize before duplicate lookup, retained bucket count after shutdown, and no
  propagation-based pick priority. Shared-endpoint city cost changed98 to93;
  native remains90. No correction was fitted to that price.
- Wired read-only NT33 renderer slot61 with two literal zero arguments, lot slot6,
  terrain slots6/17/30/34, zoning and shipped tuning observations. Renderer QI is scoped.
  Explicit float/ST0 terrain slot6 boundary avoids the SDK's uint32 declaration.
- Bounded selection still refuses occupied-cell queries; classifier/reuse and real
  subclass placement/model hosts remain incomplete. Unavailable is never treated as
  a successful clear/no-hit or placement result. No native totals are substituted.
- C ABI export changes inherited from exact6 (selection/smoothing) are included in this
  lane's ABI=true declaration. No additional exports or ABI-version bump in close7.

### Validation

Final stable source passed `qbuild.py --wt <WT> all`, session70320 collected to terminal
PASS: Rust **6927 passed, 0 failed** summed across42 result lines; clippy **0 warnings**;
native build **47/47** incremental edges; CTest **74/74**. Explicit owned-file fmt check
passed for three Rust files. Native queue: `qbuild-20261024-054305-build-ctest`.
Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`,
`fc4verify-ctest.log`. Only documentation changed after this passing source gate.

Preserved failed evidence: `build-runs/failed-r1/` contains the first all check
(Rust6927/0, clippy0, native240/240, CTest73/74). ABI conformance failed because
the corrected same-named CellWidth call collided with an SDK unused-method waiver;
source was corrected to the explicit float/ST0 boundary. That check also detected
tracked documentation/HEAD changes while queued. The corrected gate held source,
tracked files and HEAD fixed. Initial focused fixture failure is retained separately
at `build-runs/nt34-initial-failure.txt`; fixture C was corrected to the specified
post-plane boundary rather than pre-boundary equality construction.

### Real game

- [Run410](../runs/nettool-close7-r1-20261024-f31a-410.md), label
  `nettool-close7-r1-20261024-f31a`: completed **ended** at2069-01-01, nettool8/14,
  exceptions/references0 through shutdown. End date was incorrectly nested under ini;
  four unknown-config warnings were retained. Diagnostic evidence, not the required
  clean2032 verdict. Existing verdict collected; no duplicate request.
- [Run411](../runs/nettool-close7-r2-20261024-b812-411.md), label
  `nettool-close7-r2-20261024-b812`: **clean** through2032-01-20, turbo1, top-level
  end_date2032-01-01, simulation30.968s, wall45s, slot1. Nettool8/14 exact, resyncs0.
  Exceptions, reference warnings, watchdog stalls, rtc, overruns, panics, structured
  exceptions, INCOMPLETE log lines and other warnings all0. Selection diagnostic
  `complete=0` is explicitly uncovered despite no INCOMPLETE warning lines.
  PreAppShutdown05:52:15.956 and PostAppShutdown05:52:15.973 precede the clean verdict
  at05:52:16.269. Existing completed verdict retained without resubmission.

Both runs used DLL SHA256
`1f06468e5ccb1b4e2e90a50fa1efd570ab8c0abe50ac9e24249ad05159b1465c`.
Full evidence: `build-runs/game-results/<label>.log` and `.verdict`.

All ordered dragged-cell projections agree. Six counted lines differ:

| Probe | Native/core result | Question |
|---|---|---|
| Road (4,4)-(14,4), initial | cost110/113 | NT35 |
| Diagonal road (20,4)-(28,12) | cost170/192 | NT35 |
| Shared-endpoint road (14,4)-(18,8) | cost90/93 | NT35 |
| Street (64,8)-(74,8) | cost55/61 | NT35 |
| Powerline (4,32)-(16,32) | valid13/0, cost65/0, success/failure differs | NT36 |
| Bridge road (42,43)-(58,43) | valid17/0, cost974/0, success/failure differs | NT36 |

Live powerline selection logged complete=0,prefix13,rays10,lots40. Pieces, bridges,
poles and wires remain uncovered. Inherited normal-speed
[run383](../runs/nettoolexactsix20261024b.md) was also clean and8/14 with the same six
probe categories differing (shared-endpoint core cost then98). This is prior-source
evidence, not normal-speed validation of f2ed3a386; no turbo-only difference was
established, so no further game request was submitted.

### Shutdown and ownership audit

No exact6/close7 vtable writes exist to undo. Factory-created script tools use the
documented AddRef/Shutdown/RELEASE_CREATED sequence; SC4Vector uses the game allocator.
Resources, exemplars and DB records are scoped. Renderer QI is acquired/released
locally; borrowed city services and lot lookup results are never released. Acquired
occupant queries are balanced. Registry references are released at CityShutdown on
disabled replacement paths. No persistent game objects were added by this adapter.

### Questions

Appended **NT35 and NT36** to [network-tools questions](../specs/questions/network-tools.md)
at8dd737ee9. NT35 asks for admitted live retained-piece/plane/support inputs beyond
passing NT34 fixtures; NT36 asks for occupied-cell/classifier and subclass/model host
integration contracts beyond recording fixtures. Earlier NT28/NT29 remain observation
history. Coordinator regenerates the question index; this lane did not execute tools/
scripts or edit the generated index. No coverage-map or default-enable promotion.

## Handoff

- No outstanding game request, wait session or build operation remains.
- Ready queue is for measured improvements with documented gaps, not completion of14/14.
- Next: obtain NT35/NT36 answers, connect admitted live inputs and real hosts, then rerun
  the14 counted probes and shutdown check before promotion.
- If a difference is shown to occur only under turbo, run corrected source at normal speed.
- Intentional detached HEAD: supervisor publishes the reserved branch; no shared ref changes.
- Queue uses actual full HEAD, kind engine, abi=true and run410/run411 tags.

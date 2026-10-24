| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-watercraft-close12 | `core-watercraft-close12` / engine lane, detached reserved worktree | in progress; resuming `18be8b1e9`; ABI unchanged | Finish settled owner integration and vectors; run all and queued verify; commit report and READY entry. |

## Handoff

- Goal: cSC4Watercraft closure in `engine/core/src/watercraft/`, except spec-open WC10/WC11.
- Prior implementation: `7fc6ee83f`; fixture/format corrections: `015caaa14`; native pass: `d23d651e4`.
- Prior verify passed 81/81; prior all interrupted, not passed. Evidence remains in build-runs.
- Current batch: core wandering picker, picker-to-AI integration, player-drive/far-docking/state/animation orchestration in `runtime.rs`, concrete crash-effect refresh, six ordered wake-stop updates, six integration tests in `runtime_tests.rs`.
- Formatting: seven owned files pass (`build-runs/fmt-r2-final.log`).
- Checkpoint commit: `5e9b9abae` adds this round's implementation and six tests.
- Focused native cargo operation 49453 was cancelled and collected to terminal exit 1 because source advanced after its compilation started; `build-runs/focused-r2-1.log` is retained, not a test verdict.
- First all operation 10089: test FAIL 7142/1 (stale wake-stop fixture); clippy FAIL two excessive-precision spec-vector literals. Corrected the fixture to include six specified refreshes; documented verbatim decimal vectors with a scoped lint allowance.
- Failed logs archived as `build-runs/failed-r2-1-qbuild-test.log` and `failed-r2-1-qbuild-clippy.log`; summary `all-r2-final-1.log` retained.
- Both original all and explicit verify are waiting on native queue: operations 10089 / 56577, labels `qbuild-20261024-182755-build-ctest` / `qbuild-20261024-182822-build-ctest`. Collect both to terminal; do not duplicate them.
- Guard reached two-thirds at context 101135/150000: stop new feature work, finish validation/corrections and report partial closure honestly.
- Next: collect both original native waits, then repeat all on corrected stable source (real test/clippy diagnostics justify it), commit final report and queue entry.
- WC12 already filed; inspect current answers before adding questions.
- No game request. No plugin/capi/ABI changes. WC12 remains open; README's other pending rows remain implementation debt.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-watercraft-close12-r2-manifest.json`.

## Previous round evidence

This round adds reviewable helpers but **does not achieve cSC4Watercraft owner closure**.
The user-set two-thirds budget stop was reached before the full Rust gate finished.
All operations were collected to terminal results; no gate or game request remains outstanding.
The merge queue entry is WIP, not READY.

Implemented column module paths, relative to `engine/core/src/watercraft/`:

- `targets.rs`: AI midpoint consumption/strict-distance gates, asymmetric turn targets,
  signed/zero/negative-time arithmetic, 3D water-level translation, direct publication,
  snapshot shifts and slew. Refill resets speed/turn on picker failure too.
  The picker hook still needs complete core orchestration.
- `commands.rs`: shared tween ratio rounding, wider interpolation intermediates, direct
  target on nonpositive/NaN duration, unordered ratio and infinite-endpoint behavior.
- `subsidiary.rs`: visibility/model updates, ordered animation suppression/completion,
  strict lighting thresholds/cache writes, inherited dispatch/two-time order, and binding
  to the executable shared `engine/core/src/model_reg_effects/` controller.
- `crash.rs`: strict contact gates/retention, contact/heel transform composition and
  publication order, preserved scale, separate crash/state entry points with double dt,
  wake stops, tween reset and effect prerequisites. Concrete effect refresh and common
  dispatcher completion integration remain implementation debt.
- `closure_tests.rs`: WC3-T01–T04, WC4-AI01–AI10, WC5-H01–H10, inherited dispatch,
  crash timing/effect order, visibility, suppression and lighting boundaries. Existing
  scalar heel fixture now explicitly sets the newer endpoint parameter.

The README Owner closure table adds the new sections and names remaining settled work
`pending`, rather than treating answered WC4/WC5 as spec gaps. Its implemented column
uses the module paths above. Coverage remains partial, including section 11.

## Measured validation

- Corrected native `qbuild.py --wt <WT> verify`: PASS, build 47/47 edges (302.0 s),
  CTest 81/81 (331.6 s), total 635.9 s. Native cargo summaries total **7382 passed,
  0 failed across 48 test-result lines**, aggregated from
  `engine/build-x86/Testing/Temporary/LastTest.log`.
  Summary: `build-runs/verify-corrected.log`; full logs: `build-runs/fc4verify-build.log`
  and `build-runs/fc4verify-ctest.log`. Queue tree-changed note reflects lane documentation
  and commits advancing after enqueue; Rust source was unchanged after enqueue.
- Initial native: build 262/262 PASS, CTest 80/81 FAIL. A new midpoint fixture invoked
  a three-point helper with zero points. Corrected fixture/precondition documentation.
  Full failed logs: `build-runs/failed-verify-1-build.log` and
  `build-runs/failed-verify-1-ctest.log`; summary `build-runs/verify-final.log`.
- Formatting: PASS, 8 explicit owned Rust files, zero differences,
  `build-runs/fmt-corrected.log`. Initial relative paths were invalid; the subsequent
  formatting diagnostic identified the repository's 120-column setting and was corrected.
- Required `qbuild.py --wt <WT> all`: launched once; waited for build lock, then compiled
  core/conformance/formats/capi. Interrupted and collected to terminal exit 1 at budget
  stop. **No all PASS and no Clippy verdict.** Logs: `build-runs/all-final.log` and
  `build-runs/qbuild-test.log`. Corrected source was in place before compilation began.
- Earlier focused cargo: interrupted/collected exit 1 after native tests supplied the
  actionable fixture failure; `build-runs/focused-1.log`.
- No game run: core-only work, no plugin/capi adapter or shadow. No ABI exports changed.

## Questions and remaining closure

Appended **WC12** to `docs/specs/questions/watercraft.md`, using next free ID in QUESTIONS.
Section 5.7.2 says heel B=p[25], whereas 5.7.5 and H10 require p[24]; code follows newer
completion/H10 with a comment. H09's incoming basis conflicts with explicit positive-speed
banking replacement. Code follows that banking call and retains unguarded degenerate
normalization. Contact position/order is tested; a unique H09 contact basis cannot be
certified against both statements. No answered question was copied back open.

Open sections: model initialization in 3/3.5/3.6 (WC10), inherited model publication in
5.1/10 (WC11), and newly found 5.7.2/5.7.5 contradiction (WC12).

Settled debt remains: lifecycle/LoadModels and field/interface details; complete ordered
animation/player-drive ticks; common state/crash completion integration; concrete effect
refresh/tow-rope handling; wandering picker/search and AI orchestration; far docking;
distinct station/berth positions; nullable route/neighbor boundaries; remaining conformance
vectors and shared debug lifetime. README retains section-by-section pending rows.
WC4/WC5 are answered and do not justify Unsupported settled work. Older geometry-free
methods still return legacy gap variants and need integration with the new helpers.

Commits: `7fc6ee83f` implementation/question; `015caaa14` fixture/format corrections;
`d23d651e4` native pass record. Next: finish pending owner work, reconcile WC12, run stable
all gate to terminal verdict, then enqueue READY with full HEAD.

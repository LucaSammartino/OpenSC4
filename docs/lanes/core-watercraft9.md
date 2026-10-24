| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-watercraft9: watercraft round 2 | Codex / core-watercraft9 | ready | Land measured engine work; continue pending module README rows. |

## Report

READY for the measured round 2 work in `engine/core/src/watercraft`.
The watercraft owner remains incomplete; the module README records the remaining
specified work and open contracts. No C ABI exports, capi or plugin wiring changed.

Changes:

- `63964c3d1`: replace boolean steering/fire interpretation with float channels;
  implement nonlinear target/inverse curves, single-pass cross-zero speed response,
  skier group membership and complete-transform placement, and ordered steering.
  Add shared 13-channel edge/cooldown/event bookkeeping, rendered heading/rudder,
  speed-tween sampling/reset, and explicit drive-state steering storage.
- `49bab1019`: fill neighbor/ferry routes to ten points, preserve the u32 direction
  sentinel, implement survivor-direction backtracking, inherited tween/seek math,
  scalar crash ramps, sinking orientation/heading feedback and distinct height writes,
  and compose outside-city, boat and shore collision detection in owner order.
- Encode WC8 speed/steering/toggle, WC6 fire/render/tween/bookkeeping, WC7 route,
  WC5 scalar/sinking-orientation and inherited-seek boundary vectors.

Measured validation on final source `49bab1019`:

- Focused watercraft-filter snapshots: 57 then 66 passed, zero failures. The latter
  preceded the final steering-record mirror adjustment; the final workspace gate
  covers that adjustment. Earlier focused compile errors and wrapper failure are
  preserved separately in `build-runs/focused-r2-failures.txt`.
- `qbuild fmt`: 11 explicit absolute owned Rust paths; exit 0, zero diff lines.
- One final-source `qbuild all`: **PASS**, terminal result collected from the same
  operation (session 85765). Workspace totals summed across every test-result line:
  **7008 passed, 0 failed, 0 ignored**. Clippy with `-D warnings`: **0 diagnostics**.
- Native request `qbuild-20261024-092423-build-ctest`: **PASS**, build **47/47** edges
  in 78.3 s; CTest **79/79** in 147.8 s; fc4verify **PASS** in 229.8 s.
  The shared queue delay preceded these measurements. No duplicate gate launched.
- Evidence: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-build.log`, `fc4verify-ctest.log`, and
  `build-results/qbuild-20261024-092423-build-ctest.verdict`.

Game runs: none. This core module has no runtime adapter, so no in-game shadow,
reference-warning count or native-owner equivalence claim is made. No operation
or game request remains outstanding.

Questions: **WC9**, appended to `docs/specs/questions/watercraft.md`.
For L=R=100, T=0, dt=10, §6.2.2's literal ordered predicates produce T=-0.1,
whereas WC8-08 expects -0.05 and clears L. Code and a test mark the literal choice;
WC8-08 conformance is not claimed. Prior WC3–WC5 remain open. The coordinator
should regenerate the question index; this lane did not run forbidden tools or
edit the generated index.

Next steps:

- Resolve WC9, then finish wandering search, inherited animation flag/state
  dispatch, full drive tick, crash effect/double-dt dispatch, heel/contact transforms,
  tow-effect transform/refresh, lifecycle/geometry/debug fidelity and remaining vectors.
- Add a focused runtime adapter before claiming an in-game shadow comparison.
- Supervisor publishes these detached-HEAD commits to the reserved lane branch;
  merge queue uses the full actual final HEAD and `kind=engine`, `state=ready`.

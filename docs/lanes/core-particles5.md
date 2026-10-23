| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-particles5: particle owner closure | Codex / core-particles5 | ready | Coordinator review/merge; spec writers resolve remaining PT1/PT2/PT3/PT8–PT13 domains |

## Delivered contracts

The seven requested owners extend the existing Rust core modules, with explicit
Unsupported results at the spec's unresolved boundaries. No plugin/C API wiring,
ABI exports or ABI version changed. Renderer and terrain remain foreign traits.
This is ready to merge as the settled portable contract, with the open domains
listed below; it does not claim complete native rendering or contact solving.

- Ordinary particles extend the existing emitter with full records, ordered RNG,
  creation/motion, pooling, event selection and draw-before-finish dispatch.
- Geometry sources retain independent resources/materials and preserve parameter,
  emission/physics dispatch and ordered abstract drawing contracts.
- Decals model integer handle ownership, startup failures, description/parameter
  channels, rectangle placement and strict lifetime behavior.
- Dynamic hosts model distinct entry/body handles, ownership, message conversion,
  capped substeps and sector reference transitions.
- The body helper models copied descriptions, dead reclamation, joint/sector
  unlinking, both integration lists and the supplement's primary-sector predicate.
- Sequence descriptions provide ordered primitive IO and allocation/retention;
  particle description bases retain field widths and deep-copy owned containers.

The section-by-section **Owner closure** table and test mapping are in
[the module README](../../engine/core/src/effects/particle_owners/README.md).
PO01–PO36 from particles §19.2 are exercised with fake foreign interfaces; these
are spec fixtures, not measured game golden outputs. PO22 reaches the open
transform boundary after the settled zero-time/dead-sweep prefix. PO31 integrates
both lists before reaching the open collision pass.

Round 1 milestones: `be820a12` implementation, `0ac8cd17` clippy corrections,
`7fc22e01` formatting/audit. Round 2 implementation: `e75d9397` accepts disabled
attractor curves, excludes the source derived refcount from base assignment,
and extends deep-copy, factory retention and foreign stream-failure fixtures.

## Verification

- Final stable-source gate: **qbuild all PASS**, native label
  `qbuild-20261023-225125-build-ctest`, collected to terminal on session 33176.
- Cargo: **6,461 passed, 0 failed, 0 ignored**, independently summed over every
  `test result:` line in `build-runs/qbuild-test.log`.
- Clippy: **PASS, 0 diagnostics**, `build-runs/qbuild-clippy.log`.
- Native build: **PASS 49/49 edges**, 78.0 s; C/C++ cache 32 hits, 0 misses.
- Full CTest: **PASS 69/69**, 167.1 s; native verify total 249.4 s.
- Verdict: `build-runs/build-results/qbuild-20261023-225125-build-ctest.verdict`;
  full logs: `build-runs/fc4verify-build.log`, `build-runs/fc4verify-ctest.log`.
- Focused run: **48 passed**, `build-runs/round2-focused.log`, before final
  assignment-fixture simplification; final full gate covers that refinement.
- Explicit-file formatting: **PASS**, 3 edited Rust files, 0 diff lines.
- Initial round-2 compile diagnostic preserved separately in
  `build-runs/round2-focused-initial-failure.log`; fixture corrected before gate.
- Collected old native verdict before this gate: build 220/220 and CTest 69/69
  passed, but its changed-tree warning excludes it as final evidence. Prior
  clippy failure remains in `build-runs/first-gate-clippy.log`.
- Game runs: **n/a**, core only. No outstanding build or game request.

## Spec limits and questions

No new question in round 2. This lane filed **PT12** and **PT13** in
[particles questions](../specs/questions/particles.md). Existing questions are
reused rather than duplicated. Remaining open sections:

| Sections | Explicitly open domain |
|---|---|
| §2 | PT3 runtime static constructor defaults |
| §§3, 5.1, 6, 6.1, 7, 7.1 | PT1 untraced meanings and height/force/attractor/walk/city-volume helpers; settled subdomains implemented |
| §8.3 | PT2 event-name composition |
| §§9.1, 13.2 | PT13 ordinary native service/transform/parameter inventory; PT1/PT8 configuration/render packing and camera helpers |
| §§14.1–14.3 | PT3 initial center; PT8 geometry building/dicing, model bounds, alternate rendering and nonzero wave arithmetic |
| §§15.1–15.3 | PT9 successful creation/projection, initial phase, variance/whole-city placement and complete renderer records |
| §§16.1–16.3 | PT10 radius/key/clipping, occupants, terrain response/transforms/shading; PT11 joint numeric operation |
| §§17.1–17.3 | PT11 mass/extents, angle/cache/friction, contact response/pass order and joints; PT12 conflicting primary-sector spec |

PT12 is handled literally: the existing rigid-body helper's minimum remains
unchanged, while the particles supplement's maximum is a separate predicate.
Neither native solver is replaced. §18.1 sequence IO and §18.2 base assignment
have no new open behavior; stream failures propagate without invented rollback.

Next: coordinator review/merge, then revisit listed domains when their questions
are answered. Do not claim pixel, terrain or collision equivalence from this gate.

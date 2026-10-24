| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lot-models5: Rust lot model owners | Codex; core-lot-models5 | ready | Coordinator review and land; retain explicit spec-open exclusions |

## Report

Extended the existing pure Rust lot-model owners to close the settled contracts
in `docs/specs/lot-models.md`. The [Owner closure table](../../engine/core/src/lot_models/README.md)
maps every applicable section to code and tests, with spec-open exclusions below.
No plugin/capi wiring, C ABI export, ABI version, game file or coverage-map change.

Changes and implementation commits:

- `0c4b6b530`: basic zero-initialized unsigned 16-bit refcount and wraparound,
  QI identities/failure-output preservation, owner release-before-acquire,
  custom data/default fields and ordered binding/vertex/owner destruction.
  Cells retain raw invalid-orientation UVs before repeated mirroring, initialize
  shared render outputs, and retain equal-pointer bindings while releasing the
  extra acquisition and refreshing cached alpha metadata. Foundations expose
  the empty sentinel/default scale, unchecked zero/negative scale UVs, signed
  existing counts and packed BGRA draw lighting.
- `cfa7ee3e0`: zero wall enable flag skips drawing; enabling still requires
  facing. Literal lattice conversions accept signed and large finite inputs;
  wall shifts/traversal enforce the spec's finite domain. Reversed foundation
  border ranges sample nothing rather than failing validation.
- `99557a765`: cell texture loader returns true for disabled/successful bindings
  and false for two failures; Draw retains its specified null-binding behavior.
- `083e15f71`: foundation depth uses signed32(N << 4), including low-word
  wraparound after valid conversion. Literal N=134217729 produces base 4F000000.

## Measured checks

- Final `qbuild all`: **PASS**, Cargo **6768 passed, 0 failed, 0 ignored**, summed
  over all test-result lines; Clippy **0 diagnostics** with warnings denied.
  Includes **55 lot-model tests**, the section 8 vectors, four sides × eleven
  literal wall ledgers, and section 8.2 ordered cell traces/counterexamples.
- Final all native request `qbuild-20261024-005348-build-ctest`: **PASS**,
  build **50/50 edges** (117.6 s), CTest **69/69** (149.6 s), total 271.5 s.
- Explicit requested `qbuild verify`, request
  `qbuild-20261024-011050-build-ctest`: **PASS**, build **49/49 edges** (1.7 s),
  CTest **69/69** (76.9 s), total 80.5 s. No tree-change notice on either final run.
- Final `qbuild fmt`: **PASS**, seven explicit owned Rust files, zero differences.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-build.log`, `fc4verify-ctest.log` (native logs contain latest verify).
  All asynchronous checks were collected to terminal results.
- Earlier evidence is superseded: initial all passed 6766 tests/Clippy and
  69/69 native tests but reported a changing tree. The section audit then fixed
  actual source, justifying the final full check. Summary retained in
  `build-runs/gate-initial-source.txt`. Focused milestones passed 51 then 54
  tests. The initial q.py launch failed before Cargo due to Bash mapping access;
  its full log remains `build-runs/q/20261024-003008-21760.log`.

## Questions and limits

No new questions filed. Existing [LM1, LM2 and LM8](../specs/questions/lot-models.md)
remain open exactly where the spec is unsettled:

- **§1.2 / LM2**, all owners: remaining native/inherited inventory, slot 11,
  unpublished constructor fields, allocation failure and native pool/layout.
  Published §1.1 lifetime is implemented; this is not an 82-body completion claim.
- **§4 / §4.3 / LM1**, cell: upstream authors, bypass-service identity and
  meanings of other mode bytes. Published explicit bypass input and modes 0–3/255
  are implemented; other mode semantics remain unsupported.
- **§7 / LM8**, cross-cutting exceptional numeric/input/service domains and
  ambient x87 precision/rounding beyond the documented PC53/nearest-even model.
  Published raw orientation retention, unchecked foundation scale division,
  signed low-word shift and literal finite lattice rules are implemented.

Game runs: **n/a**; pure Rust scope has no plugin/capi wiring or game shadow.
No native object layout, pool allocation, GPU pixels or real-game equivalence
is claimed. Native owner references use explicit callbacks and stable Arc
identities; acquired binding leases are released once, and getters borrow.

Next: coordinator review/land and claim owner coverage with the explicit open
contracts above. Future work on those contracts depends on spec answers.

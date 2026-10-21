| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-disasters4 (`enable = disasters`: native cSC4DisasterLayer replacement glue) | `glue-disasters4` / (engine lane) | blocked: layer boundary contract absent in disasters.md; DS42-DS44 filed; no code changed, no ABI change, no game run | (1) spec writers answer DS42 (how the city creates the layer: GZCOM factory or direct, IID, refs, teardown), DS43 (cached layer global `0x00B43D3C` writer/readers), DS44 (is cISC4DisasterLayer the whole boundary); (2) then a layer-only `cFC4DisasterLayer` over native managers created via `CreateInstance(class, 0x021E9CBF)` (§2.2) |

## Report

### What was read
disasters.md §0-§3 (lines 24-366), foundations.md §3.6, docs/abi rows for cSC4DisasterLayer, FC4Subsystems.cpp class
table, FC4DisasterHost.h, the capi `fc4_disasters_*` list.

### Finding (why blocked)
- An `enable = disasters` replacement is a class-factory registration (FC4Subsystems.cpp `ClassEntry`). disasters.md §1
  gives the layer's class `0x61F6ABF5` but does not say that the city creates it through GZCOM (other specs do, e.g.
  flammability.md §1 `GetClassObject(0xE990C045, ...)`), nor the IID, the references the city holds, or the teardown
  calls. Registering a factory on that guess is the "guessed class registration" the brief forbids. -> DS42.
- §2.3 names a cached global layer pointer `0x00B43D3C` read by native code; its writer is unspecified, so a
  replacement cannot tell whether native readers would see null or a stale pointer. -> DS43.
- Whether the interface is the whole boundary (QI for `0x452294AA`, direct field reads) is unspecified. -> DS44.
- Design once answered: a layer-only `cFC4DisasterLayer` (layer state of §2: counters, damage assessment, timers,
  cost record, locations) that creates the native managers via `CreateInstance(class, 0x021E9CBF)` (§2.2) and returns
  them borrowed from slot 7. The Rust `fc4_disasters_*` core bundles Rust managers; using it as-is would need
  C++ wrapper managers (the "dummy child managers" the brief forbids), so the layer-only part may need a small
  layer-only capi surface (core layer.rs already isolates §2).

### Measured
No code change; the baseline `qbuild all` was started but not needed (no diff). No game request sent.

### Questions
DS42, DS43, DS44 in docs/specs/questions/disasters.md.

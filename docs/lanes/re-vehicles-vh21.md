| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-vehicles-vh21 (vehicle/train/tank specs) | Codex / `re-vehicles-vh21` (RE; detached HEAD) | ready; doccheck passed; VH5 answered; VH1 partial; ABI unchanged | 1. Land queued specification changes. 2. Finish remaining Vehicle geometry under VH1. |

## Report
- Output is specification text and conformance vectors in vehicles.md §§12.10–12.15; no engine, plugin, C ABI, tools or coverage-map edits.
- Vehicle: command/steering accumulation, signed speed, grade/tween rules, time-based substeps and collision scheduling. Corrected measured-speed receiver to automaton +0x94.
- Train/TrainCar: composition, SetLeader chain, following/opacity, trip/accident/failure, overspeed/derail, distance-based substeps, head anchors, path switching and collision order.
- Tank: instruction-verified yaw/acos/rate, pivot, elevation/power/history arithmetic, sound timing, shot attributes, power×0.01 scaling, gauges and channel-7 cooldown.
- Concrete-owner method/vtable inventory, inherited dispatch, construction, drawing and IO boundaries link existing automata specifications.
- VH5 answered DIFFERS in questions/answered/vehicles.md (2026-10-25). VH1 narrowed; VH7 unchanged. No new questions; questions index regenerated.
- Real-work commits: cef6d6d8f, 6e0658efa, 5289b5e16, c61b0c739, and this final specification/report commit.
- Gate: `python tools/doccheck.py check` PASS; `git diff --check` PASS. Cargo/clippy/ctest n/a; no game runs or shadow comparison.
- q.py failed to launch Bash (CreateFileMapping error) before doccheck ran; corrected-source direct PowerShell gate passed. Commit succeeded despite a packed-refs lock warning.
- Evidence: main export first, bounded main decomp context, instruction bytes and PE constants/vtables. REA unused (no binary open); no ABI row proposals.
- Gate log: build-runs/doccheck-re-vehicles-vh21-final.log. Read-only PE helper: build-runs/pe-vehicles.py. Queue: build-runs/merge-queue/re-vehicles-vh21.json (ready, full HEAD).

## Handoff
- VH1 remains OPEN solely for Vehicle on-network displacement/reattachment and off-road anchor/steering geometry in 0x005603F0/0x0055ED10; full VH1 closure was not achieved.
- Projectile damage is behind the effects interface; the Tank owner supplies transform/attributes and makes no direct damage call. Do not invent damage from unscaled power.
- Coordinator can land the ready documentation queue; implementation may use the answered contracts while retaining the explicitly open Vehicle geometry boundary.

# Maintained repository inventory

Current entry points and their dependencies, audited 2026-10-25. Detailed command syntax stays in the linked
workflow documents. Historical snapshots and retired experiments live in Git history, without archive copies.

| Area | What remains | Purpose and current consumers |
|---|---|---|
| Tools: orchestration | `conductor.py`, `lanes.py`, `lane_launch.py`, `lane_backend.py`, `lane_refill.py`, `lane_heartbeat.py`, `lane_prompt.mjs` | Coordinator run loop, lane reservation/launch/monitoring, prompt rendering; [coordinating](coordinating.md), [providers](lane-providers.md), [commands](../lane-tools.md), `.claude/workflows/fc4-lane.js`. |
| Tools: landing and builds | `merge-train.py`, `bump_abi.py`, `build_queue.py`, `creditcheck.py` | Batch integration, ABI numbering, queued native builds and measured lane credit; [merge train](../merge-train.md), `.claude/lane-bin/qbuild.py` and `fc4verify.py`. |
| Tools: game runs | `game_queue.ps1`, `ingame_run.ps1`, `regress.ps1`, `regress_shadows.ps1`, `city_fixture.ps1`, `sc4ctl.ps1`, `sc4mutex.ps1`, `run_summary.ps1` | Queue, stage, drive and summarise isolated game runs; [game workflow](running-the-game.md); merge-train regress. |
| Tools: run comparisons | `clock_diff.ps1`, `compare_soaks.ps1` | Documented clock/long-run differential recipes in [engine build/test](../../engine/README.md) and [manual checks](../manual-checks.md). |
| Tools: coverage and ledger | `coverage.py`, `coverage_treemap.py`, `ledger.py`, `compare_metrics.py`, `function_boundary.py`, `oracle_class.py`; `coverage-*.tsv` | Byte attribution, treemap, per-function status and comparison evidence; [coverage](../COVERAGE.md), [ledger](../ledger.md), merge-train credit checks. |
| Tools: documentation | `doccheck.py`, `gen_handoff_tables.py`, `questions.py` | Instruction checks, lane/run tables, question index; tools tests, merge-train regeneration and [coordinator workflow](coordinating.md). |
| Tools: ABI | `abi_extract.py`, `abi_functions.py`, `abi_returns.py`, `abi_validate.py`, `abi_direct_contracts.json` | Published byte-derived ABI extraction and validation; [ABI procedure](../abi/README.md). |
| Tools: RE queries/data | `re_query.py`, `x86dis.py`, `addr2func.py`, `class_vtables.py`, `dbpf.py`, `extract_tuning.py` | Read-only executable/package queries and tuning extraction; [RE workflow](../re-workspace.md), specs and ABI extractors. |
| Tools: RE pipeline | `build.ps1`, `setup_ghidra.ps1`, `symport/` | Symbol matching and headless Ghidra setup/export; [RE workflow](../re-workspace.md). |
| Tools: tests | `tests/` and fixtures | Maintained tooling regressions: `python -m pytest tools/tests -q`. Native/Rust gate implementation is `.claude/lane-bin/fc4verify.py`. |
| Docs: contracts | `specs/`, `abi/` | Behaviour specs, open/answered questions and published interfaces/layouts; engine/RE briefs, conformance and ABI tooling. |
| Docs: operations | `ops/` | Rules, conventions, lessons, coordinator/provider procedures, game and compare workflows; lane prompts and coordinator. This page replaces the former HANDOFF directory index. |
| Docs: lane/run inputs | `lanes/`, `runs/` | Lane state, run measurements and record format; `lanes.py` checks/briefs and `gen_handoff_tables.py` enumerate these directories. `lanes/INDEX.md` and `manual-checks.md` are generated views. |
| Docs: current state | `HANDOFF.md`, `assignments.tsv`, `lane-tools.md`, `merge-train.md` | Coordinator state and active reservations, lane commands and landing contract. Lane read limits still apply. |
| Docs: measurements | `COVERAGE.md`, `coverage-report.md`, `coverage-map.svg`, `ledger.md`, `PROGRESS.md` | Coverage definitions, generated measurements, ledger format and a current progress pointer; README and coordinator. |
| Docs: architecture/harness | `ARCHITECTURE.md`, `harness.md` | Roadmap, system layout and plugin/compare switches; README, engine build/test and [compare workflow](compare.md). |
| Docs: RE references | `re-workspace.md`, `subsystems/`, `sim-engine.md`, `sdk-errata-draft.md` | RE workflow, subsystem evidence and SDK corrections cited by specs/sources; engine clean-room exclusions remain. |
| Docs: regression references | `incidents/train65-landvalue-timing.md`, `reviews/pr5-7-2026-10-22.md` | Incident evidence cited by the landvalue questions and review findings cited by the coordinator's PR6/7 lane. Uncited completed reports were deleted. |
| Docs: history | `history/token-ledger.md` | The only retained history file: an active measurement output linked by [coordinating](coordinating.md) and produced by `lanes.py ledger --md`. |

The audit covers every top-level Python/PowerShell tool, every docs folder, and tracked files in `engine/tests/`,
`re/`, `ghidra/scripts/` and `build-runs/`. Raw file inventory, incoming references, last commit dates and deletion
sizes are in the prune27 worktree's `build-runs/prune27/`; the lane report `docs/lanes/prune27.md` records acceptance
checks and retained uncertainties. Run evidence cited by `coverage-shadows.tsv` or specs remains available.

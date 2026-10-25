| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-vehicles-vh21 (vehicle/train/tank specs) | Codex / `re-vehicles-vh21` (RE; detached HEAD) | continue; guard context stop; VH5 answered, VH1 partial; doccheck passed before final audit; ABI unchanged | 1. Correct SetLeader wording first. 2. Finish VH1 and doccheck. |

## Report
- Spec-only changes in vehicles.md §§12.10–12.14; no engine, plugin, C ABI or coverage-map edits.
- Vehicle command/steering, signed speed, coasting, grade and tween rules; Train composition, following, opacity, trip and failure contracts; Train overspeed/cache/braking/derail; Tank shot attributes and cooldown; concrete-owner method/slot inventory.
- Added 44 conformance table vectors across the four new behavioural subsections; corrected accident, random-sound, derail and Tank elevation summaries.
- VH5 moved to questions/answered/vehicles.md as DIFFERS (2026-10-25); VH1 narrowed in questions/vehicles.md; VH7 unchanged; questions index regenerated. No new question IDs.
- Sources: main export queried first, main decomp for context, instruction bytes for numeric branches and PE constants/vtables. No REA calls or fallback errors; REA has no binary open as supplied.
- Gate: `python tools/doccheck.py check` passed; `git diff --check` passed. Cargo/clippy/ctest n/a for this RE-only change; no game runs or shadow comparison.
- Full VH1 closure is missing: Vehicle/Train motion/substeps/junctions/collision order, final Tank aim/power/elevation and sound instruction audit, projectile-effect damage consumer.
- Local ignored evidence/helper: build-runs/pe-vehicles.py; doccheck log build-runs/doccheck-re-vehicles-vh21.log. No ABI row proposals.

## Handoff
- Goal: finish instruction-backed Vehicle/Train/Tank owner specs and close VH1/VH5. No queue entry: guard stopped before a mandatory correction; do not merge this tip yet.
- FIRST fix vehicles.md §12.11: creation and following incorrectly call slot 49 a speed copy. It is SetLeader (automata.md §11). Creation sets previous car/head as leader; following rebuilds the preceding-object chain, not speed values. Add the chain vector (would make 45 vectors).
- Evidence already checked: 0055AA26–0055AA2D pushes previous object to slot49; following 0055A83F–0055A854 pushes previous object from stack+68, then 0055A861 updates that previous pointer to current car. This supersedes the inherited handoff's mistaken speed-copy interpretation. The attempted corrective patch was DENIED and changed no files.
- Then finish narrowed VH1: `asm 005603f0`, `asm 0055ed10`, remaining movement pages of `asm 005591a0`, final aiming/sound pages of `asm 0054b030` / `asm 0054aae0`, using --bytes and bounded pages; identify effect damage consumer.
- Committed real work: cef6d6d8f commands/speed; 6e0658efa VH5; 5289b5e16 Train/Tank/inventory/report. Full real-work tip 5289b5e162be527d665ccf6eebc2e3ef3da9569a.
- Gate passed before final layout-only edit; rerun doccheck and diff --check after the SetLeader fix. Questions index regenerated; VH1 remains open, VH5 moved, VH7 unchanged. No game runs, ABI proposals or REA use.

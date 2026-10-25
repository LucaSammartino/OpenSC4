| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-vehicles-vh21 (vehicle/train/tank specs) | Codex / `re-vehicles-vh21` (RE; detached HEAD) | ready for spec merge; VH5 answered, VH1 partial; doccheck passed; ABI unchanged | 1. Complete narrowed VH1 audit. 2. Implement from specs in core lane. |

## Report
- Spec-only changes in vehicles.md §§12.10–12.14; no engine, plugin, C ABI or coverage-map edits.
- Vehicle command/steering, signed speed, coasting, grade and tween rules; Train composition, following, opacity, trip and failure contracts; Train overspeed/cache/braking/derail; Tank shot attributes and cooldown; concrete-owner method/slot inventory.
- Added 45 conformance table vectors across the four new behavioural subsections; corrected accident, random-sound, derail and Tank elevation summaries.
- VH5 moved to questions/answered/vehicles.md as DIFFERS (2026-10-25); VH1 narrowed in questions/vehicles.md; VH7 unchanged; questions index regenerated. No new question IDs.
- Sources: main export queried first, main decomp for context, instruction bytes for numeric branches and PE constants/vtables. No REA calls or fallback errors; REA has no binary open as supplied.
- Gate: `python tools/doccheck.py check` passed; `git diff --check` passed. Cargo/clippy/ctest n/a for this RE-only change; no game runs or shadow comparison.
- Full VH1 closure is missing: Vehicle/Train motion/substeps/junctions/collision order, final Tank aim/power/elevation and sound instruction audit, projectile-effect damage consumer.
- Local ignored evidence/helper: build-runs/pe-vehicles.py; doccheck log build-runs/doccheck-re-vehicles-vh21.log. No ABI row proposals.

## Handoff
- Start with open VH1's narrowed scope; its original entry remains open deliberately. Do not treat the shot effect's unscaled power as damage or claim complete driving physics.
- Exact next evidence: query `asm 005603f0`, `asm 0055ed10`, remaining movement pages of `asm 005591a0`, and final aiming/sound pages of `asm 0054b030` / `asm 0054aae0`, using --bytes and bounded pages.
- Existing completed ownership/records/defaults/route geometry remain §§12.1–12.9. Train numeric oddities and persistent following scratch are compatibility rules, not corrections to make in core.
- First substantive commits this round: cef6d6d8f (commands/speed), 6e0658efa (VH5); final commit adds Train/Tank/inventory and this report.

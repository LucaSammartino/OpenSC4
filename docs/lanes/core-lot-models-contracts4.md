| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lot-models-contracts4: settled LM3–7 | Codex then Claude, `core-lot-models-contracts4` | ready (gated) | Merge train; LM1/LM2/LM8 stay open in questions/lot-models.md. |

## Report
- Module: `engine/core/src/lot_models/` implements the settled LM3–7 contracts of `docs/specs/lot-models.md`
  in the existing foundation, retaining-wall and cell-texture owners: foundation quads, border enumeration,
  fallback keys and BGRA recolour; wall biased cell ranges, raw query order, three emit branches, world UVs,
  retained tails and recolour; cell strip/orientation tables, zoom/rotation updates, mode classification
  (mode 255 = no mode; any other byte, including 0, is tinted), packed states and the modes 0–3 combiner bytes.
- Resource leases are Rust-owned: borrowing releases nothing, replacement/drop releases once.
- Conformance vectors lm01–lm06 corrected to the settled spec; lm05 now uses mode 255 as the absent mode and
  has an explicit mode-0 tinted step (this was the earlier gate failure).
- Gate (qbuild all, final source): cargo 5598 passed / 0 failed; clippy -D warnings 0 diagnostics; native build
  217/217; ctest 67/68 with `plugin_ini_turbo_test` failing on wall-clock pacing under parallel-lane load
  (lane touches no plugin code); that test passed on an isolated rerun. qbuild fmt: 7 files, 0 diff lines.
- Not done: LM1 (upstream authors, other mode bytes), LM2 (object/base/QI/pool/lifetime), LM8 residual
  arithmetic; no ABI, plugin or capi wiring; no in-game run; no native/GPU equivalence claim.

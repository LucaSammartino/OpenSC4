| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-networkmodel4 (cSC4NetworkOccupantModelInstance core, network-occupants.md §10) | `core-networkmodel4` / (engine lane) | ready at `8372f704` + report: core only, no C ABI or plugin change; gate cargo 4035/0 (baseline 4029), clippy clean, ctest 65/65; no game run (nothing the game loads changed); questions NO21-NO23 filed | 1. When NO21-NO23 are answered, adjust `DeformFrame`, `corner_height` and the `ModelRenderer` passes. 2. A glue lane wires `GetModelInstances` (slot 33) to `PieceRender` and a renderer provider. |

## Report

**What changed.** `engine/core/src/network_occupants/model.rs` (new) and `tests_model.rs` (new), wired in `mod.rs`.

- What is complete (the published §10 contracts): the four overrides (`ModelOverride`), slot 33 `GetModelInstances`
  (`PieceRender::model_instance`), `Deform` as a corner bend in network-geometry.md §8.1 corner order
  `(x,z),(x,z+1),(x+1,z+1),(x+1,z)`, the flip bit (bit 7 of `+0x104`) handed to `FlipAndLight`, and model-or-fallback
  selection (the §2.2 quad and triangles when there is no model). Conformance test 24 is covered.
- What is left, because the spec does not publish it: the bodies of `FlipAndLight`, `Clip` and `Prelight` go through
  a typed `ModelRenderer` provider. The flat model comes from a typed `ModelProvider`. The core never calls an
  original model-instance method for an output.
- Choices, marked in the code: the bend is bilinear, the deformed y is the flat y plus the bend, every call starts
  again from the flat model, the frame has its origin at (0,0) with a 16 m extent, there is no clamping, and the
  piece has no instance when it has no model.

**Measured.** Baseline gate: cargo 4029/0, clippy 0, ctest 65/65. Final gate at `8372f704`: cargo 4035/0 (6 new tests),
clippy 0, ctest 65/65.

**Questions.** NO21 (Deform: interpolation, additive or replaced y, local frame, clamping), NO22 (what the
flip, lighting, clip and pre-light passes do, and their order), NO23 (instance lifetime, how it relates to the §2.1
`cS3DModelInstanceBasic` sub-object, what slot 33 returns with no model). All three are in
docs/specs/questions/network-occupants.md.

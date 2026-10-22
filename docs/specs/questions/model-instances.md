# model-instances: open questions
<!-- Questions about [model-instances.md](../model-instances.md). Append new entries at the END; when one is answered, move it to answered/model-instances.md. Rules: README.md. -->

<!-- from: re-modelinst (2026-10-20) -->
- **MI1 - §6.1:** what the five header values after a sprite animation's type tag, and the value after its five
  zoom keys, hold; nothing in the reader interprets them, and the sprite instance was not traced reading them.
  [kept, not interpreted]

<!-- from: re-modelinst (2026-10-20) -->
- **MI2 - §4:** what `LightModel`'s `flip` argument is for (it negates the rotation used to light the normals) and
  which callers pass true. [as read]


<!-- from: core-model-instances4 (2026-10-22) -->
- **MI4 - §2 world bounds:** is the transformed model box the enclosing AABB of all eight corners, including negative uniform scale and non-symmetric matrices? [literal enclosing AABB; preserve empty boxes]

<!-- from: core-model-instances4 (2026-10-22) -->
- **MI5 - §2 Pick/view-utilities §7:** is nearest hit distance the signed triangle ray parameter t, or Euclidean distance to the intersection for non-unit directions and intersections behind the origin? [literal signed t; the geometry ray test explicitly accepts t < 0]




<!-- from: core-model-instances4 (2026-10-22) -->
- **MI9 - §§2–4 malformed/numeric inputs:** are non-finite playback/transform inputs and invalid primitive vertex/index references in scope? [preserve literal f32 comparisons/arithmetic, no FMA; unordered animation magnitude takes the one-step catch-up branch; under-sized primitive triangle counts saturate to zero and invalid mesh references are skipped for memory safety, consistent with graphics-formats' safe reader choices]


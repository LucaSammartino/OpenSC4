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
- **MI6 - §§2,4 material/processor lifetime:** are SetModel's first-material mutations confined to the instance copy; does processor SetModel return true when a non-format-2 mesh leaves it with only the instance; and do explicit ReleaseModel/destruction restore original vertices? [instance copy; true for a retained instance, false only for explicitly refused multi-frame animation; restore only at the documented first step of SetModel, ReleaseModel/destruction release all references exactly once]

<!-- from: core-model-instances4 (2026-10-22) -->
- **MI7 - §6.1 upward sheet pass:** what happens to initial own-instance keys before any non-I source exists (including all-I keys); does encountering a non-I source preserve the downward pass's scale? [preserve downward results until a source exists; retain scales of non-I keys because the upward pass only assigns a scale to I keys; retain opaque MI1 values]

<!-- from: core-model-instances4 (2026-10-22) -->
- **MI8 - §4 repeated SortMeshes:** are ties stable against container order on each call or against the preceding processor permutation? [restart from container order each call, then literal descending stable insertion sort, including unordered comparisons]

<!-- from: core-model-instances4 (2026-10-22) -->
- **MI9 - §§2–4 malformed/numeric inputs:** are non-finite playback/transform inputs and invalid primitive vertex/index references in scope? [preserve literal f32 comparisons/arithmetic, no FMA; unordered animation magnitude takes the one-step catch-up branch; under-sized primitive triangle counts saturate to zero and invalid mesh references are skipped for memory safety, consistent with graphics-formats' safe reader choices]


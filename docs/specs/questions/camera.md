# camera: open questions
<!-- Questions about [camera.md](../camera.md). Append new entries at the END; when one is answered, move it to answered/camera.md. Rules: README.md. -->

<!-- from: re-camera (2026-10-19) -->
- **CA1 - §2.5:** what the 0.48 factor and the `lift` term of the scroll-bounds polygon correspond to on screen
  (the polygon's top edge is raised by `o.y × 0.48 / viewRow1.y` minus half the viewport height in metres).

<!-- from: core-camera-control4 (2026-10-22) -->
- **CA3 - §1.4, §7 perspective entry (residual after re-camera-contracts4, 2026-10-22):** identify a native perspective box setter/call site, if one exists in this Windows build, and prove its argument order and counter side effects. [Type-2 evaluation is proven, but no setter is identified in the native owner inventory; pure-core `set_frustum` remains an extension without native setter/counter fidelity. Resolved portions are normative in §§1.1–1.4: box −1…+1, near 1/far 2, remembered target/up both zero, unchecked degenerate normalization, byte transform versus u32 projection counters. The original provisional choices were box −0.5…0.5 and remembered up Y; those differ.]

<!-- from: core-camera-contractfix4 (2026-10-22) -->
- **CA7 - §2.1 pre-init derived conversion storage:** what are the constructor values (or defined observation limits before a camera update) for the camera-plane equation, target's camera-plane point and conversion scale? [The public state table resolves direction/right zero and up Y, but does not specify these derived fields. Pure core provisionally uses a zero equation/point and unit conversion scale, marked in code; no whole-owner or native pre-init conversion fidelity is claimed.]

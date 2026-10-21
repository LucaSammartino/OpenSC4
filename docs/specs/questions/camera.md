# camera: open questions
<!-- Questions about [camera.md](../camera.md). Append new entries at the END; when one is answered, move it to answered/camera.md. Rules: README.md. -->

<!-- from: re-camera (2026-10-19) -->
- **CA1 - §2.5:** what the 0.48 factor and the `lift` term of the scroll-bounds polygon correspond to on screen
  (the polygon's top edge is raised by `o.y × 0.48 / viewRow1.y` minus half the viewport height in metres).

<!-- from: re-camera (2026-10-19) -->
- **CA2 - §1.1, §1.4:** the depth bias at `+0x1A4` of `cS3DCamera`: nothing in this lane sets it. Which caller does,
  and to what?

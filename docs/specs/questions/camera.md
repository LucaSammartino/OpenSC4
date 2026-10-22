# camera: open questions
<!-- Questions about [camera.md](../camera.md). Append new entries at the END; when one is answered, move it to answered/camera.md. Rules: README.md. -->

<!-- from: re-camera (2026-10-19) -->
- **CA1 - §2.5:** what the 0.48 factor and the `lift` term of the scroll-bounds polygon correspond to on screen
  (the polygon's top edge is raised by `o.y × 0.48 / viewRow1.y` minus half the viewport height in metres).

<!-- from: core-camera-control4 (2026-10-22) -->
- **CA3 - §1 defaults, degenerate look-at and perspective entry:** what are the exact inferred unit-box endpoints and initial remembered target/up; does zero-length/parallel look-at normalize without a fallback; what is the perspective box setter and its counter contract? [Use centred unit box −0.5…0.5, near 1/far 2, target zero/up Y; literal reciprocal-length normalization propagates IEEE NaN; pure-core `set_frustum` mirrors SetOrtho with type 2. No new ABI is inferred.]

<!-- from: core-camera-control4 (2026-10-22) -->
- **CA4 - §2 lifecycle and save edges:** what are unspecified pre-init derived vectors/clip distances, Shutdown behaviour, malformed saved zoom/rotation handling, partial-read state, and NaN SetZoom behaviour? [Zero derived vectors/clip distances with identity axes/scale until computed; pure-core shutdown only detaches/clears initialised; retain record indices and bound table access only, use zero corner for an invalid rotation; Read commits atomically only after a complete good version-1 record, with no city/camera access; refuse NaN zoom. These choices are explicitly marked in code.]

<!-- from: core-camera-control4 (2026-10-22) -->
- **CA5 - §2.6 command details and §2.7 messages:** does integer SetZoomAndRotation/SetRotation reset a custom magnification, does SetCameraPositionTweakAdd replace or accumulate the tweak, and does SetViewportSize recompute projection/position/bounds? Are change messages synchronous callbacks or ordered posts? [Literal “the same” integer zoom transition resets magnification to 1; unchanged integer pair does nothing, unchanged rotation returns true; tweak arguments replace the additive pixel offset; viewport setter updates the stored/camera viewport only as stated. Pure-core events capture previous/new pairs in order for a later adapter; no synchronous observer is assumed.]

<!-- from: core-camera-control4 (2026-10-22) -->
- **CA6 - §2.5 inferred slide rule, outside starts and tiny edges:** how is a zero-length move from an outside target brought inside; is the 1e−8 edge cutoff a length or squared length; is 0.0001 subtracted from the fraction or a world distance; what are tie/winding and failed-ground-intersection rules? [Use literal edge length, fraction minus 0.0001 when fraction ≥0.001, first edge on equal fractions, polygon signed-area winding; for an outside start project onto an offending edge line at fraction zero and slide the remaining move along it. At most edge-count iterations, then retain the original from. Ground failure retains target and emits no target-change event. The already-published 0.48/lift formula is used unchanged; CA1 is not a formula blocker.]

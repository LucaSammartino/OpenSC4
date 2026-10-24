# intro-video: open questions
<!-- Questions about [intro-video.md](../intro-video.md). Append new entries at the END; when one is answered, move it to answered/intro-video.md. Rules: README.md. -->

<!-- from: re-introvid7 (2026-10-23) -->
- **IV1 - §4:** `QueueMovie` calls the resource manager's slot 37 `(key, &resource, 0)`, then wraps the serial record
  in GZCOM class `0x0A32C0DA` (IID `0x8A32C0AC`) and queues that object's slot 5 result. Is slot 37 "open the
  resource" and the class a record stream for the decoder? [inferred]

<!-- from: re-introvid7 (2026-10-23) -->
- **IV2 - §3:** `GZPaint` calls the graphics system's (`0xC47B747C`) slot 3, then slots 12 `(0, 0, 0, 0)`, 15 `(1)`
  and 11 `(0x4000)`. Is that a clear of the frame to black? [inferred]

# advisors: open questions
<!-- Questions about [advisors.md](../advisors.md). Append new entries at the END; when one is answered, move it to answered/advisors.md. Rules: README.md. -->
<!-- from: re-advisor-ui4 (2026-10-21) -->
- **AD6 - section 8.3 (camera_jump / camera_zoom):** these two are simulation-collector entries (`0x40AF90`,
  `0x40B1F0`), not UI ones, and §8.3 gives no behaviour. What are their arguments, results, the 3D-view / camera
  calls they make and whether they read `0x00B207C0`? [found while answering AD5; a replacement registering all
  47 entries needs them]

# sound-renderer: open questions
<!-- Questions about [sound-renderer.md](../sound-renderer.md). Append new entries at the END; when one is answered, move it to answered/sound-renderer.md. Rules: README.md. -->

<!-- from: re-sndrender (2026-10-22) -->
- **SRN1 - section 4.5:** what the 1–6 level at `+0xB8` of a 2D sound controls (a streaming chunk count, a quality
  level or a voice priority). [voice priority]

<!-- from: re-sndrender (2026-10-22) -->
- **SRN2 - section 5:** which parameter the 0–127 value scaled by 1/127 sets on the 3D helper (cone outside volume,
  Doppler or roll-off). [roll-off]

<!-- from: re-sndrender (2026-10-22) -->
- **SRN3 - section 4.5:** the play-position formula of `0x00876255` (it multiplies the sample count by 8000 before
  dividing by the rate, then by 1000 when stopped). [an ms value with a unit slip]

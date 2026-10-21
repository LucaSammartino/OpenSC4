# legacy-control: answered questions

<!-- from: re-legacycontrol4 (2026-10-21) -->
- **LC6 - section 9.1:** contract of helper 0x0094C1DA (SetGZWinImage, 4 stack arguments) called after an image is set. [unknown]
  **ANSWERED 2026-10-21 (re-legacycontrol4):** DIFFERS — it stores {group, instance} as an 8-byte type-0x8000 variant in control parameter 0xDAA3B860 + n (Slider n = 0); see section 9.4.

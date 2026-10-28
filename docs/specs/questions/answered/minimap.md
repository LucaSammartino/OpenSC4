# minimap: answered questions (archive)
<!-- Archive for [minimap.md](../../minimap.md). Lanes do not read this file. Open questions are in ../minimap.md. -->

<!-- from: re-minimap7 (2026-10-23) -->
- ~~**MMW4 - §1, §7:** the surface service `0xC416025C` and the lock codes `0x8001`, `0x8040`, `0x8010`, and the
  surface format `(9, 32)` given at creation. [inferred: a 32-bit ARGB surface, read/write and read-only locks]~~
  **ANSWERED 2026-10-27 (re-x-ui):** DIFFERS — `0xC416025C` is the graphic system and `(9, 32)` is colour type 9 (32-bit A8R8G8B8) at 32 bits; the lock codes are caller-chosen bit masks that the buffer only records in a flag word (with a second counter for bit `0x8000`), with no read-only or read/write distinction (Init, section 7).

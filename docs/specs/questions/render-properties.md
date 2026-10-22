# render-properties: open questions
<!-- Questions about [render-properties.md](../render-properties.md). Append new entries at the END; when one is answered, move it to answered/render-properties.md. Rules: README.md. -->

<!-- from: re-renderprops (2026-10-19) -->
- **RP1 - §1.2, §3:** who reads the two extra values of an integer or float entry (`+0x20`, `+0x24`; for example
  1–10 for `kParticleEffectsLOD`, 0–5 for the backing-store zooms)? None of the 18 slots does, and set ignores them.

<!-- from: re-renderprops (2026-10-19) -->
- **RP2 - §1.2:** what the two extra bytes of a boolean entry (`+0x1D`, `+0x1E`, always 0 at registration) are for.

<!-- from: re-render-properties-contract4 (2026-10-22) -->
- **RP6 - §1.3 non-C text profiles:** Which LC_CTYPE/codepage profiles occur at render-properties construction and later Find calls in each shipped language, and what are the resulting comparisons for ASCII I/i and bytes `0x80–0xFF` before NUL (including a locale change between construction and lookup)? [The owner demonstrably calls active-locale CRT tolower on sign-extended bytes, not Unicode folding; the language manager can change the locale. §1.3 and §7.2 establish the C-locale ASCII core domain only. Need locale/codepage-controlled native discriminators before extending that claim; no universal non-ASCII miss or fixed Windows encoding is asserted.]

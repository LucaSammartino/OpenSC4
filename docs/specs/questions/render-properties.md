# render-properties: open questions
<!-- Questions about [render-properties.md](../render-properties.md). Append new entries at the END; when one is answered, move it to answered/render-properties.md. Rules: README.md. -->

<!-- from: re-renderprops (2026-10-19) -->
- **RP1 - §1.2, §3:** who reads the two extra values of an integer or float entry (`+0x20`, `+0x24`; for example
  1–10 for `kParticleEffectsLOD`, 0–5 for the backing-store zooms)? None of the 18 slots does, and set ignores them.

<!-- from: re-renderprops (2026-10-19) -->
- **RP2 - §1.2:** what the two extra bytes of a boolean entry (`+0x1D`, `+0x1E`, always 0 at registration) are for.

<!-- from: core-render-properties4 (2026-10-22) -->
- **RP3 - §1.1, §2 slots 0–2:** Please specify the initial reference count, accepted QueryInterface IDs (including whether only IID 1 is accepted), failed-query output handling, and Release-at-zero/deleting-destructor behavior for this owner's two interface views. The SDK supplies interface signatures and a cRZUnknown example but does not establish the game's complete owner lifecycle. [Core implements independent construction/table destruction through Rust ownership; COM slots 0–2 remain unimplemented pending the owner contract.]

<!-- from: core-render-properties4 (2026-10-22) -->
- **RP4 - §2 slots 6–11, 15–17:** The original performs unchecked indexing. Is any behavior beyond valid indices intended as a compatibility requirement (negative signed i32, index equal to count, extreme i32), or is this outside the contract? [Safe core returns None for invalid getters/names and false for invalid setters without mutation; no original out-of-bounds result is claimed.]

<!-- from: core-render-properties4 (2026-10-22) -->
- **RP5 - §1.3, §2 Find:** Which byte encoding/locale defines query lowercasing, and are query strings terminated at the first NUL? [Core accepts Rust strings, folds ASCII only, and compares the entire input; all 77 registered ASCII names and short names are supported. Non-ASCII folding and C-string truncation are not claimed.]

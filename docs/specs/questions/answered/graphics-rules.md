# graphics-rules: answered questions

<!-- from: re-graphicsrules (2026-10-19) -->
- **GR2 - §4.2:** is `stringMatch` case-insensitive (`WildcardMatch` with its third argument 0)?

  **ANSWERED 2026-10-22 (re-graphics-rules-completion4):** MATCHES — zero lowercases copies of both
  arguments before the whole-string `*`/`?` matcher. See §4.2, `0x00918F08–0x00918F18`.

<!-- from: re-graphicsrules (2026-10-19) -->
- **GR3 - §7:** the conditions of the built-in `standard software` and `standard hardware` rules, and the threshold
  of `two stages`.

  **ANSWERED 2026-10-22 (re-graphics-rules-completion4):** DIFFERS — software requires hardwareDriver clear;
  hardware is unconditional; its depth-1 child requires textureStages ≥ signed integer 2. All are non-partial,
  all-condition rules. See §7 and native stores `0x007D143B–0x007D144A`, `0x007D1782–0x007D17AA`.

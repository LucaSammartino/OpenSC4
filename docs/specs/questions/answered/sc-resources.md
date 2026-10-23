# sc-resources: answered questions

<!-- from: re-persist-sl5 (2026-10-23) -->
- ~~**SCR1 (§2):** What are the exact term/function grammar and remainder rule,
  and mixed-delimiter nesting/escape behavior? Enum lookup and color/vector
  contracts are now specified in §2.3. Inspect `0x0067AC30`,
  `0x0067B990`; verify numeric
  operations against instructions before publishing constants/rounding.
  [unsettled; only operator precedence and binary64 accumulation specified]~~

  **ANSWERED 2026-10-23 (re-scres-scr5):** DIFFERS — sections 2.1–2.2 specify
  same-delimiter nesting, no escaping, comment truncation before grouping,
  exact function names, signed-factor/right-associative power and floor-based
  remainder. Numeric widths and constants have instruction anchors; vectors
  SCR-V31–37 distinguish these rules from conventional parsing.

<!-- from: re-persist-sl5 (2026-10-23) -->
- ~~**SCR2 (§3):** Complete cCommandParser method-name/default-state audit,
  built-in command argument/failure contracts and variable insertion boundary
  parentheses. Anchors `0x00675F50`, `0x00677B90`, `0x00678B00`.
  [unsettled; published tables retain unknown semantic names]~~

  **ANSWERED 2026-10-23 (re-scres-scr5):** DIFFERS — sections 3.1–3.3 name
  all slots, distinguish construction/Init/Reset, specify builtin counts and
  failures, prefix fallback, exact-key removal and whitespace-only wrapping
  with a whole-argument exception. The literal setc component rotation is
  documented as a quirk; vectors SCR-V38–48 cover the closed boundaries.

<!-- from: re-persist-sl5 (2026-10-23) -->
- ~~**SCR3 (§4):** Settle include relative-path resolution and recovery after
  nested parse errors. Watcher message, slot 11 and include literal/open-error
  policy are now specified in §4. [unsettled]~~

  **ANSWERED 2026-10-23 (re-scres-scr5):** MATCHES — section 4.2 joins relative
  includes to the containing file's parent directory without canonicalization.
  A nested error unwinds the enclosing file; only the next top-level registered
  file is attempted. Optional syntax errors record diagnostics without forcing
  aggregate failure. Vectors SCR-V49–52 specify paths, recovery and hooks.

<!-- from: re-persist-sl5 (2026-10-23) -->
- ~~**SCR4 (§§5–6):** Complete token wire type-number table
  and binary adapter boundary. Descriptor comparison, QI/defaults and the
  final-byte counter rule are now specified in §§5–6. [type 3 is uint32;
  other type assignments and binary field payload encoding unsettled]~~

  **ANSWERED 2026-10-23 (re-scres-scr5):** DIFFERS — section 5.2 maps all
  twelve wire types and delegates serial encoding to persistence section 3.
  Fixed-width reads reject equality after advancing; strings delegate before
  testing <=. Section 6.1 records that the text uint64 read slot inherits a
  binary eight-byte read. Vectors SCR-V53–60 cover the adapter boundary.

<!-- from: re-persist-sl5 (2026-10-23) -->
- ~~**SCR5 (§7):** Settle record BMP offset handling and exact 4444 dithering
  arithmetic/rounding (`0x0097822A`, `0x0097844B`, `0x00978E35`). Failed-load
  retained-buffer state and malformed RLE limits are not currently defined.
  [unsettled; use the documented supported conversion pairs only]~~

  **ANSWERED 2026-10-23 (re-scres-scr5):** DIFFERS — sections 7.2–7.3 specify
  the record path's offset check without seeking, zero-size RLE length,
  nontransactional retained-buffer failures and exact integer serpentine
  7/3/5/1 nibble-error diffusion. Malformed native RLE has unsafe bounds,
  not a deterministic successful result contract. Vectors SCR-V64–68 cover
  supported input; BMP lock flags are corrected to 0x8040.

<!-- from: core-sc-resources5 (2026-10-23) -->
- ~~**SCR7 (§§2–3,5–7):** Please settle the remaining portable boundaries:
  decimal operand/exponent grammar; variable namespace lookup order and `%`
  parameter boundaries; whether SetTokenString names the field definition;
  how length-prefixed SetFieldString fragments are consumed by the raw text
  delimiter reader; runtime numeric overflow/nonfinite spelling; the color-type
  number selecting 4444 and the image resource's base type. [Minimal decimal
  literals; newest namespace prefix first; whole-token `%name`; token string
  used as definition name; preserve literal SetFieldString writes and raw
  delimiter reads separately; numeric prefixes use caller-width wrapping;
  no claimed text roundtrip or 4444 parity; image base type zero pending a
  specified type. Existing explicit open domains remain SCR1–SCR5.]~~

  **ANSWERED 2026-10-23 (re-scres-scr5):** DIFFERS — sections 2.2, 3.2,
  5.2, 6.2 and 7 specify permissive decimal scanning with a wrapping integer
  exponent helper, embedded parameter substitution, parent-prefix lookup,
  definition-name assignment, raw length-prefix consumption, modular integer
  scanf input and legacy CRT nonfinite spelling. Color type 7 selects 4444;
  the base resource type is key.type, initially zero. Vectors SCR-V34–42,
  SCR-V56–63 and SCR-V68 close these portable boundaries.

<!-- from: re-persist-sl5 (2026-10-23) -->
- ~~**SCR6 (§8):** Audit constructor defaults, empty-stack OutputStream behavior,
  array separators, complete graph/reference literals, and surveyor creation
  failure. Existing scalar and graph order facts are not a byte-complete
  ASCII grammar (`0x00982F1C`, `0x009835ED`).~~

  **ANSWERED 2026-10-23 (re-persist-sl5):** MATCHES — section 8 specifies
  zero initial reference count, active-surveyor stream access, array trailing
  comma-space, graph/reference/type-info literals and LF ordering. Factory
  success is unchecked before surveyor use; there is no native successful
  result contract for a missing surveyor or empty stack. Float rendering is
  delegated to the typed output stream, not assigned invented precision here.

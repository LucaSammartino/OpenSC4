# sc-resources: answered questions

<!-- from: glue-scres-shadow5 (2026-10-24) -->
- **SCR8 - native capture ABI for sections 3-8:** Please publish complete x86 signatures (including return channels and concrete versus interface views) for command/file parser input and output tokens, token-record initialization and typed getters/setters, text-adapter string/definition writes, buffer-resource read/write and buffer access, and ASCII writer fields/graph/stream access. The behavior/slot inventories do not establish these argument types. In particular the text numeric writers have only one stack word even for f64/s64: are these const references? What non-mutating accessor can capture the original raw record bytes and ASCII output without changing its cursor or invoking serialization again? [Keep those capture paths off; implement only the specified adapter numeric/descriptor/key reads, with SDK read/seek operations restoring the original position. Partial adapter coverage must not credit the whole subsystem.]

  **ANSWERED 2026-10-24 (re-scres-scr8):** DIFFERS — section 10.1 publishes
  stack arguments, AL/EAX/void channels and object views. Token adapter numeric
  writes are const references, including f64/s64; ASCII f64 is by value.
  Typed token accessors take descriptor/value references and an element index.
  ASCII string slots 22/23 are object/char-pointer respectively. Neither token
  record nor ASCII writer offers a universal raw-byte snapshot getter: observe
  existing raw IO/sink writes, or use the guarded read/seek fallback in 10.2
  with exact cursor restoration. Vectors SCR-V69–73 and SCR-V75–79 apply.

<!-- from: glue-scres-shadow5 (2026-10-24) -->
- **SCR9 - natural resource/token capture dispatch:** In real-city run `scres5-r1-2032-a` (Konradshohe, turbo through 2032-01-19), the specified binary/text adapter vtables installed successfully before PreAppInit, but every numeric/descriptor/key slot had zero observations (73 dated lines, `scres 0/73`, no refusal or native difference). Which natural resource-file paths use these tables, which call implementations directly, and which earlier lifecycle/capture entry is required to see the game's actual parser/token/image/writer traffic? Please specify the complete argument/result ABI and safe passive capture route for those paths. [Try installation at OnStart before framework initialization; never credit an empty sample or substitute constructed inputs for real file traffic.]

  Follow-up evidence (2026-10-24): `scres5-r1-2032-b` installed at OnStart before framework initialization, corrected the ini registration and load date, and reached 2032-01-05 cleanly with no warnings and ref_warnings=0. Still `scres 0/73`: calls=0, bytes=0, refused=0 throughout. Earlier lifecycle installation did not expose natural adapter dispatch. No native value differences were observed because no comparisons occurred. The lane remains WIP without coverage credit.

  Round-2 implementation note (2026-10-24): the implementer cannot resolve SCR8/SCR9 from decompilation without violating the binding clean-room role. `scres_constructed` now calls only the already specified numeric/descriptor/key adapter read ABI on fixed section-6 vectors and a shipped pack-1 INI byte prefix (key published in feature-manager section 4.1). This is partial adapter coverage, not a command/file/INI parser shadow or an answer establishing natural dispatch. Please publish the remaining signatures in the permitted spec/ABI sources; no question is considered answered by the constructed probes.

  Round-2 measurement (2026-10-24): `scres5-r2-constructed-birch` reached 2032-01-07 cleanly with ref_warnings=0. Constructed binary adapters 24/24, text adapters 22/22, and a shipped pack-1 INI's first 16 raw bytes through binary scalar adapters 10/10 were exact (56/56, refused=0, mismatches=0; harness scres 3/3 lines). Key 00000000/8A5971C5/2B563701, prefix FNV-1a 6AE1A633. This establishes only the stated adapter cases, not natural dispatch or complete command/file/resource parsing. SCR8/SCR9 remain unresolved.

  **ANSWERED 2026-10-24 (re-scres-scr8):** DIFFERS — sections 10.1–10.2
  specify startup rule-file/command dispatch, token-compatible factory read
  and write, direct image loading and ASCII graph output with native capture
  boundaries. Rule files use cRZTextFile and parser input slot 5, not token
  adapters; macro/eval argument dispatch and image decode include direct calls.
  Install parser capture before rule initialization, not merely token tables
  before city dates. No lifecycle forces all owners to execute; empty or
  constructed samples cannot establish natural owner coverage. Vectors
  SCR-V74–79 distinguish these routes. The preceding historical unresolved
  status describes the implementation round before this specification.

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

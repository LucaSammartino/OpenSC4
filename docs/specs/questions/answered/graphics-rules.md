# graphics-rules: answered questions

<!-- from: replace-cfgrules28 (2026-10-26) -->
- **GR9 — §§2.1/2.2 native provider integration:** Please publish the interface-only acquisition contract needed to bind the manager in the real game: (1) the forced-FSAA probe's render target, drawn geometry/vertex format, state setup/restoration and readback-surface acquisition/format, in addition to the already specified twenty iterations and 64 sample positions; (2) an interface getter for backing-store presence (the spec currently names a private global); (3) the version/OS provider and optional sound interface IDs/slots, including the first six strings' formatting and sound-name getters. The SDK supplies GS/current-driver/record and directory/preference interfaces, but no complete acquisition route for these remaining inputs. A constant false probe, guessed backing-store flag, native manager/log-output callback or direct call to the original private helper would not establish the specified independent provider boundary. The private transport now accepts explicit measurements and refuses unavailable acquisition before Init mutates state. Native factory registration, rule-query off/on comparison and acceptance runs are stopped pending this contract; existing GR4/GR6/GR7/GR8 supported-profile limits also remain.

  **ANSWERED 2026-10-26 (re-gr29):** DIFFERS — §§2.3–2.6 publish the independent acquisition routes and
  worked fixtures. The probe uses the current UI color buffer, format-1/16-byte vertices, twenty quad
  draws and a driver-QI color-buffer copy, then restores UI defaults. Backing-store enabledness is not
  presence: use the clamping setter/getter and restore the saved byte; an uninitialized renderer from
  the public factory works before a view exists. There is no version-service IID; use the specified
  Win32/CPU measurements and executable resource strings independently. Sound service 0xE9D27715,
  IID 0x49D28666, slot 25 fills both string references. No manager/private-helper delegation is needed.
  This answers acquisition, not game acceptance or the remaining GR4/GR6/GR7/GR8 limits.

<!-- from: re-graphicsrules (2026-10-19) -->
- **GR1 - §2:** the exact sources of the flags `DXT`, `fullscreen` and `forcedFSAA` (`0x008D2320`) and whether
  `bogoFPS` and `bogoMPPS` are ever measured.

  **ANSWERED 2026-10-22 (re-graphics-rules-completion4):** DIFFERS — DXT/fullscreen read current driver record
  bytes +0x2C/+0x20; forcedFSAA is a hardware-gated rendering/readback probe returning true for more than one
  nonblack sample on its 8×8 grid. Both bogo values stay constructor-initialized +0.0; there is no benchmark
  assignment in this manager. The probe is performed but does not measure bogo values. See §§2–2.1.

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

<!-- from: core-graphics-rules4 (2026-10-22) -->
- **GR5 — §4.2 versus vectors V1 NaN contradiction:** §4.2 says NaN environment numbers pass all-mode numeric tests; V1 says a NaN "passes neither numeric condition in all mode". Which result is normative? [Follow §4.2: all-mode NaN passes, any-mode NaN supplies no match; mark the conflicting V1 edge explicitly.]

  **ANSWERED 2026-10-22 (re-graphics-rules-final4):** MATCHES — all-mode NaN passes each numeric
  condition; any-mode NaN supplies no numeric match. The contradictory V1 wording is corrected.
  Native x87 masks/parity branches at `0x007CE940–0x007CE943`, `0x007CE97A–0x007CE97D`,
  `0x007CEAC2–0x007CEAC5`, `0x007CEAFC–0x007CEAFF` decide unordered explicitly; see §4.2.

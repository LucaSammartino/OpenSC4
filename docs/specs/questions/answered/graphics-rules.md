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

<!-- from: replace-cfgrules28 (2026-10-26) -->
- **GR10 — native rule-file path acquisition/search:** Section 3 names a data directory and a user-data directory, but does not specify the native acquisition calls or complete path construction. Which service/interface IID and slots (and arguments) obtain the bases used by ParseConfigFile, and what suffixes/search behavior produce the required Graphics Rules.sgr and optional Graphics Options.sgr paths? Include the automatic config-log directory as a separate case. Application §9 slot 51 supplies `.../Apps/` and slot 56 supplies `.../Plugins/` in the measured installation; literal concatenation fails to open both files and installs fallback (Init status 1). The original/replacement query runs have 20 different final render properties out of 77, so this is not sufficient parity evidence. Evidence: main `build-runs/game-results/cfgrules28-r2-queries-on1.log` lines 131–133; off/on logs with the same label stems. Please distinguish direct file opens from any game filesystem/resource-parser search rules; the implementation will not search guessed folders or hard-code an installation layout. Provider acquisition §2.3–§2.6 is implemented; this path item is stopped pending the contract.

  **ANSWERED 2026-10-26 (re-gr30):** DIFFERS — §§3.1 and 6.2 specify `cISC4App` IID
  `0x26CE01C0`: required rules use slot 53 (`+0xD4`, output string, language −1), optional
  options use slot 58 (`+0xE8`, output string), automatic logs use slot 51 (`+0xCC`, output
  string). Append the respective filename directly; logs append stored computerName and
  `-config-log.txt`. `nSCRes::cFileParser` (class `0x5CF9459A`, IID `0x1CF94583`) registers
  rules with flags 1, then options with flags 0, and opens only those full paths in that order.
  No alternate-base search occurs: duplicate rules select the slot-53 copy, duplicate options
  the slot-58 copy; a copy only in the other base is ignored. Missing rules fail aggregate
  parsing and cause fallback even when options exist; missing options alone are ignored.
  Availability cases and worked paths are in §3.1; no new game/parity result is claimed.

<!-- from: replace-cfgrules30 (2026-10-26) -->
- **GR11 — native rendering/caller state after independent Init:** Does the manager's Init/Gather/probe sequence have a required renderer, driver or UI state-restoration contract beyond §§2.3–2.4, or another native caller contract beyond the manager IID? Please specify any missing interface-only steps; no native layout or extra restore is being guessed.
  - Source `afc66170e`, DLL SHA256 prefix `f2e48878`: GR10 roots open the required rules and included card file; Init status=0. Main `build-runs/game-results/cfgrules30-r1-queries-{off2,on2}.log` measure final properties 77/77 and recommendations 12/12 exact. The key-off owner shadow has 13/13 exact snapshots (77 properties and 12 recommendations each); both opposite-key save loads and s1-grow controls are clean/ref_warnings=0.
  - House list plus cfgrules: `cfgrules30-r1-house2.log:9293` records access violation reading null at `SimCity 4.exe+0xE2C06` on 2026-01-27, before simulation advances. Its stack has no return into the plugin. Both `cfgrules30-r1-house-on3.verdict` and the same-DLL key-off control `cfgrules30-r1-house-off3.verdict` are clean through 2032-01-01/ref_warnings=0, so (a) has a passing run but the initial fault remains unexplained.
  - s0-grow with cfgrules: `cfgrules30-r1-s0-grow-on1.log` fails at the same `+0xE2C06` on 2026-01-30; `cfgrules30-r1-s0-grow-on2.log:9294` repeats it on 2026-01-27. The same script without cfgrules (`cfgrules30-r1-s0-grow-off1.log:11409`) instead fails at `+0x3529C0` on 2026-03-13. All have ref_warnings=0. These are distinct from the task's known quit-time construction-occupant Shutdown fault.
  - The s0 portion of acceptance (c) is held on this unexplained difference. The logs above are in the main checkout's game-results directory. No rendering-state, private-memory, caller-layout or exception-address behavior is inferred from them.

  **ANSWERED 2026-10-26 (re-gr31):** DIFFERS — §2.4 corrects the root acquisition to
  `cIGZWinMgr` service `0xA417445E`, IID `0x000005A4`, slot 3; §2.7 makes borrowed versus counted
  GS/driver/window/view/renderer references explicit. No further saved-state restore or
  application-specific manager IID is required by the examined native paths. Native Init is void,
  gathers/parses, and is followed by a separate slot-4 evaluation; it does not own a view lifecycle.
  Three worked interface cases cover live-view lifetime, the uninitialized renderer facade, and
  repeated Init. The `+0xE2C06` fault reads a null shared view reference in an audio-ambience
  condition before `GetCurrentViewLevel`, rather than entering the driver. The first writer/cause
  in the supplied runs is not statically settled: §2.7 specifies paired replay, view publication/
  Shutdown and acquisition-release traces, and the first null-reference writer's stack needed to
  settle it. The distinct key-off terrain fault does not supply a restoration rule. This closes
  the specification question by narrowing crash attribution to that runtime observation; s0
  acceptance and a crash fix are not claimed.

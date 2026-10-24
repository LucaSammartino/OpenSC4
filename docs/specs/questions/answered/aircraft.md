# aircraft: answered questions

<!-- from: core-helicopter9 (2026-10-24) -->
- ~~**AF9 - §4.1 rotor preset and post-Read reconstruction (AF4 follow-up):** what are the thirteen initial
  rotor preset values (exact binary32 bits), and which lifecycle step restores them and the flight phase
  after Read? The resolved zero-overlay loader requires an existing preset, but §4 does not specify one.
  [core stores unknown constructor/post-Read fields as None, accepts an explicitly supplied preset, and
  overlays known nonzero fields; zero/missing records preserve every existing field without inventing defaults]~~

  **ANSWERED 2026-10-24 (re-aircraft-af10):** DIFFERS — aircraft.md §4.1.1
  tabulates all thirteen exact preset words. InitInstance(0) copies them once,
  applies player then AI parameters; the zero-player-speed fallback is yaw maximum
  400, not preset MaxSpeed 227. Read does not restore rotor parameters or phase;
  Init reconstructs the model and PhysicsInit selects phase 0 with engine off.

<!-- from: re-aircraft (2026-10-21) -->
- ~~**AF1 - §3.1, §4:** which exemplar property holds the 18-value drive-parameter list, and what each entry means
  (speeds, turn rates, climb angles). [as read]~~

  **ANSWERED 2026-10-23 (re-aircraft-af5):** DIFFERS — aircraft.md §§3.1/4.1:
  Lua automata-group `airplane` (18 named fields) and `helicopter` (13 named fields),
  with whole-record ancestor lookup; no numeric exemplar property. Field tables and conversions are specified.

<!-- from: core-aircraft5 (2026-10-23) -->
- ~~**AF7 - §3.1, parameter-list edge cases:** what happens with a short/empty list or non-finite entries, and
  are parent chains guaranteed acyclic? Are the separately printed no-list decimal defaults exact constants,
  or recomputed from their parents? [literal finite-list implementation: missing entries act as zero;
  an existing list, including empty, stops inheritance; explicit zero uses the derived arithmetic, while no
  list uses the printed constants; no undocumented clamping or rescaling]~~

  **ANSWERED 2026-10-23 (re-aircraft-af5):** DIFFERS — aircraft.md §3.1:
  named missing fields import as zero; a present empty table stops inheritance. There is no raw short-list
  contract or cycle guard. Non-finite fields are not sanitized. No-record defaults are exact binary32
  constants (bits tabulated), not the previously rounded printed decimals; zero-derived fields use arithmetic.

<!-- from: core-aircraft5 (2026-10-23) -->
- ~~**AF8 - §5/§8 test 3, station unit mask width:** what happens when a fire-plane station's unit count
  exceeds 32, so `2^n - 1` exceeds the available-units u32 property's width? Is there an exemplar limit,
  wider property type, clamp, or wrapping rule? [core property initialization handles 1–32 exactly,
  ignores zero/missing counts and non-building occupants, and returns Unsupported(AF8) before mutation
  for larger counts; station membership/call numbering remain foreign to the flight owners]~~

  **ANSWERED 2026-10-23 (re-aircraft-af5):** DIFFERS — aircraft.md §5:
  positive signed count; availability mask uses a 32-bit shift with count modulo 32 and wrapping decrement.
  Count 32 gives zero, count 33 gives one. There is no clamp or wider availability property.

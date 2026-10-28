# commercial-industrial: open questions
<!-- Append new questions at the end; numbering comes from QUESTIONS.md. -->

<!-- from: verify-industrial4 (2026-10-22) -->
- **IN3 - sections 1, 2.3, 8.2, 9-10: original industrial natural-call forwarding boundary:** What supported public acquisition/interception and lifetime contract lets an independent industrial shadow capture the city/store-owned original instance before its first `Read`, forward each actual `QueryInterface`/reference/lifecycle/message/persistence call exactly once, and retain coverage through shutdown and final release? The published primary/message/serializable slots and successful-QI reference rule establish callable interfaces. With the industrial replacement disabled, public GZCOM `GetClassObject(0xA990C022, 0x477A9E7D, ...)` can supply a separately constructed object; city getter slot 60 supplies the city component. Neither contract describes an observer/forwarder installed before the store creates and reads that component (foundations sections 3.6-3.7: `Read` precedes city-init notifications). With the replacement registered, that same GZCOM request may resolve to our Rust-backed plugin, so it cannot independently establish a native oracle. Please publish a supported native-provider/factory forwarding route or a safe public-interface interception contract, including installation timing, identity checks, reference ownership, nested calls, and removal relative to final release/module teardown. If method-table interception is supported, specify its safe boundary without requiring private native fields or additional executable inspection. [Blocked: no industrial shadow implementation, comparisons, or exactness credit; no native table mutation or guessed provider bypass. Constructed probes alone would not verify the required natural calls.]

<!-- from: growth-exact33 (2026-10-28) -->
- **G33-CM1 — §§3–4: repeatable vehicle float differences with commercial alone replaced.**
  Does commercial's lifecycle or daily/idle processing have an observable floating-point-environment effect
  beyond the documented grid/state changes (including the unused timer in §4.3)? On `timbuktu-rush-hour`,
  `qcheck --replace commercial` twice gives the same 49 differing bytes in vehicle record
  `896E714A/299B2D1B/0` (four entries' checksums and floating-point values; first payload difference at byte 88).
  All other 151 records, digests and sampled random states agree. `--replace none` on the same scenario is exact.
  Expected vehicle SHA256: `5dc596fe9c1f466d9aa2c980d0def4673f71eb37108faf49687e4bf11d180508`;
  actual both times: `6c0db89d36b50b8cb2ca60ffdc339098021f4bc07485bd930feb4ed5a1295147`.
  Runs under `build-runs/golden-runs/`: `compare-20261028-020554-ca3c0b7f` and
  `compare-20261028-020949-f9fa67d3`; both clean, ref_warnings=0, logged x87 PC=24 during simulation.
  Please specify any such effect or other missing observable behavior; no vehicle-byte compensation was added.

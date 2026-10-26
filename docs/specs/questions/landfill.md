# Landfill questions

No open questions. LG31's published-contract decision is recorded in
[answered/landfill.md](answered/landfill.md); independent native zone verification remains unsupported
through the published public interfaces (landfill §7.4).

<!-- from: replace-lotocc28 (2026-10-26) -->
- **LG-lotocc28-1 — replacement tile exemplar lifetime:** §§4.8.1/7.3 specify that attaching an
  exemplar takes one reference and SetTileID mutates its shared key. With a replacement tile holding
  a GetResource reference, is that reference sufficient across native occupant-manager removal,
  renderer queries and surviving tiles, including cache effects of GetKey/SetKey? Please specify
  the supported ownership/lifetime contract at these boundaries. Shared queue run
  `lotocc28-landfill-r2-2032a` (MAIN/build-runs/game-results, DLL sha256 prefix `1ac88b8a`)
  stopped corrupt at 2026-03-01: `foreign_exceptions=1`, `ref_warnings=3`. Its log lines 4072–4075
  report last-reference release and two over-releases of exemplar `1607BD94` at
  `fc4::props::Exemplar::~Exemplar`, followed by an invalid read. The preceding month reports
  12 inserted and 8 removed tiles. Our adapter retains a GetResource acquisition, borrows its
  holder, and releases that acquisition at tile destruction; no additional reference or native
  field access has been guessed. Replacement tile activation remains unverified pending this
  contract; the failure has not been attributed to native construction Shutdown.
  Follow-up `lotocc28-landfill-r5-2032a` (DLL sha256 prefix `4811e717`) used the exact
  developer-loaded resource with an explicit attachment AddRef instead of a second
  GetResource lookup (4.8.1 step 3). Focused tests verify one retained reference per
  tile, repeated attachment, removal and surviving inherited-group queries. The
  game still stopped corrupt on 2026-03-01 with `foreign_exceptions=1`,
  `ref_warnings=3`; log lines 4070-4073 again show last-reference release and two
  over-releases at `fc4::props::Exemplar::~Exemplar`, now for `15F4B214`.
  This correction did not resolve the boundary. Factory registration was removed
  again; no further reference-count adjustment or lifetime guess was made.

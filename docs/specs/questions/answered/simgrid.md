# simgrid: answered questions

<!-- from: glue-simgrid-lots-shadow5 (2026-10-23) -->
- ~~**SG3 - sections 1, 5, 6.2 and 6.4 (complete live-grid inventory for record shadows):** what is the
  supported, complete enumeration contract for the original city's grids at PostCityInit and each month?
  Please specify either the exhaustive owner/getter (or owner-record/reference) inventory, with element
  type and borrowing/lifetime rules, or a public enumeration interface with its native signature and
  ownership rules. Section 6.2 says owners construct grids directly and the map view constructs additional
  uint16/int16 grids; section 5 defines records but not how to discover every live grid. Are map-view,
  transient, newly created and owner-replaced grids included, or is the requested shadow explicitly scoped
  to a defined set of persisted simulator grids? The existing plugin probe observes only references passing
  through FreeCity4's IStreamAdapter::ReadRef, not every original owner's reads or direct constructions;
  its observed set cannot establish completeness. [blocked: no new shadow or verified-coverage claim;
  do not guess a class-store iterator or treat the probe's observed subset as the full inventory]~~

  **ANSWERED 2026-10-23 (re-simgrid-sg3):** DIFFERS — simgrid §6.2.1–§6.2.2 defines the coordinator-bounded
  simulation-owner/getter inventory, types, dimensions, selectors and lifetimes, with six conformance vectors
  in §10. Re-enumerate at PostCityInit and every month, including current replacement owners and plumbing's
  swapped published object. Map-view, transient, newly synthesized/scratch and owner-private grids are excluded;
  this does not claim all saved or live grids. The shipped inventory has 86 roles. Single-pointer getters borrow;
  residential's population/EQ/HQ triples AddRef each output, and callers release each reference.

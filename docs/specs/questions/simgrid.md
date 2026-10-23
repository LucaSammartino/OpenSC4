# simgrid: open questions
<!-- Questions about [simgrid.md](../simgrid.md). Append new entries at the END; when one is answered, move it to answered/simgrid.md. Rules: README.md. -->

<!-- from: glue-simgrids4 (2026-10-21) -->
- **SG1 - section 6.3 (with sections 4 and 6.4.5):** a layout-compatible replacement of the grid class
  (`enable = simgrids`, engine/plugin/FC4SimGridLogic.h) keeps `vector<vector<T>>` in the STLport layout with
  every buffer from the STL node allocator B. Two points the spec leaves open: (a) does any original code outside
  `cSC4SimGrid<T>` ever change the vectors' structure through the `GetGridData` pointer (resize, assign, swap,
  `clear`, destroy, or keep a row pointer across a later `SetTractSize` / `Read`), or does it only read and write
  element values within `[begin, end)`? (b) Do original callers ever pass out-of-range indices to slots 7, 8, 22,
  23 or 26 in normal play (section 4: no bounds checks)? The replacement returns 0 for out-of-range reads and
  ignores out-of-range writes [choice]; if an original caller depends on reading a neighbouring row's memory,
  please name it.

<!-- from: verify-simgrids-shadow4 (2026-10-22) -->
- **SG2 - sections 2, 4, 5 and 6.2 (factory pre-initialization contract):** what public operations are valid
  on a factory-created original grid before `Init` and the first `SetTractSize`? In particular, are its geometry,
  initialized flag and `Write` record defined at this point? The spec defines sized grid behavior but does not
  state constructor defaults or the pre-initialization serialization contract. The first verification run
  (`sgv4-r1-native39-20261022a`) raised one foreign exception after the existing 117-grid probe and before any
  fixture result; no causal diagnosis from executable bytes or exception dumps was attempted. [choice] The
  constructed verifier now establishes `Init` and `SetTractSize(8)` before capturing its first input record.
  This setup receives no comparison credit; unsized factory serialization remains unverified. This is a parked
  contract question, not a request for additional RE. SG1 replacement/out-of-range choices remain unchanged.

<!-- from: glue-simgrid-lots-shadow5 (2026-10-23) -->
- **SG3 - sections 1, 5, 6.2 and 6.4 (complete live-grid inventory for record shadows):** what is the
  supported, complete enumeration contract for the original city's grids at PostCityInit and each month?
  Please specify either the exhaustive owner/getter (or owner-record/reference) inventory, with element
  type and borrowing/lifetime rules, or a public enumeration interface with its native signature and
  ownership rules. Section 6.2 says owners construct grids directly and the map view constructs additional
  uint16/int16 grids; section 5 defines records but not how to discover every live grid. Are map-view,
  transient, newly created and owner-replaced grids included, or is the requested shadow explicitly scoped
  to a defined set of persisted simulator grids? The existing plugin probe observes only references passing
  through FreeCity4's IStreamAdapter::ReadRef, not every original owner's reads or direct constructions;
  its observed set cannot establish completeness. [blocked: no new shadow or verified-coverage claim;
  do not guess a class-store iterator or treat the probe's observed subset as the full inventory]

# network-manager: open questions
<!-- Questions about [network-manager.md](../network-manager.md). Append new entries at the END; when one is answered, move it to answered/network-manager.md. Rules: README.md. -->

## From netocc-cities29 (2026-10-26)

- **NM18 — terrain removals beyond §7.6's destruction decisions (formerly filed as NM16):** in `gen-tegel-s0-default`, the shadow
  reports 14 extra original removals around cells (65,69..70), (65..66,77), (80..81,69..70). The original's
  component 53 and every component-11 lot/config/zone/wealth entry agree. The core queries with mask 0x1FFF,
  applies the kind/pipe/subway/0x80000 gates and the specified four-corner altitude test. Which native handler
  or network-tool side effect removes these pieces, and at what point should the comparison capture their
  terrain/candidate inputs? Is the §7.6 query/filter or the ownership of the resulting removals incomplete?
  `build-runs/suite/netocc-cities29-before5/gen-tegel-s0-default.log` reproduces 14 errors, netmgr exact 2/3,
  clean/ref_warnings=0. A late listener registered after city Init checks each held candidate immediately
  after 0x278E6003 / 0x6A3B0AFD. `netocc-cities29-probe1/gen-tegel-s0-default.log` still reports 14 errors;
  every listed extra removal has `placed after handler=0`. This disproves a simple later-in-the-tick removal
  explanation; moving the check alone is not justified. The diagnostic now also captures placement at the
  candidate query. This blocks the 795 terrain-detail occurrences in the supplied census, including Tegel
  and the gen27 Konradshohe/Spandau cases. No mismatch, candidate or tolerance has been removed or relaxed.

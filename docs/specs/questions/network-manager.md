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
  The current boundary probe reproduces Tegel with the shadows-profile simulator settings:
  `C:/Users/tlori/FreeCity4/build-runs/game-results/netocc-cities29-profile5-tegel.log`, clean/ref_warnings=0,
  netocc exact 2/2, netmgr exact 1/3 (16 mismatches). Every reported extra removal was placed at the query
  and absent at the late terrain listener (`placed before/after handler=1/0`). Thus already-removed query
  results and a simple later-tick check do not explain these cases.
  A separate timing case is now proved in `netocc-cities29-broad6-06.log` (gen27 Tegel grow): some pieces
  reported as original-only removals were still placed at the late terrain listener (`1/1`). The comparison
  now checks every candidate, including predicted destructions, at each handler boundary; the next-tick
  path remains the fallback for explicitly selected early-listener mode. NM18 remains about the `1/0` cases.
  With that correction, `netocc-cities29-bound8-grow.log` reports 12 rather than 17 errors, and
  `netocc-cities29-bound8-default.log` reports 10 rather than 16. Both are clean/ref_warnings=0; every
  remaining reported extra removal has `1/0`. These logs are in the main checkout's `build-runs/game-results`.

- **NM19 — Port of Shiganshina's first lot/random divergence (§§8.5, 8.7, 17.3):** the census sample
  `C:/Users/tlori/FreeCity4/build-runs/run7/census3/paradis-port-of-shiganshina.shadows.txt` first differs on
  2041-01-01: lot (1,25), config `4BA69450`, exists only in the original, with variation 1, rotation 3,
  flip 0, usable 1 and current compatibility false; zone/wealth are 0/0 in both. There are also two different
  compatible choices and generator next-word indices 386/383. Component 53 and all zone/wealth cells agree.
  After the avenue hash-order/bounds fix, which message or direct entry should first affect this edge lot,
  and which inputs must be retained before the native handler changes them? In particular, does the specified
  replay need transient forced/locked lot state not represented by component 11, or another native draw path?
  Do not infer an answer from the later incompatible configuration alone. The new diagnostic reports the
  first random disagreement's message and four data words without resyncing the generator or hiding errors.
  Fresh attempts `netocc-cities29-probe4-paradis` (corrupt during load) and `netocc-cities29-profile5-paradis`
  (stalled at the script's 2040-12-12 terrain action) cannot reach the census's first differing month;
  the latter starts with an exact netmgr load sample. This blocks that distinct census-only case, not a
  claim that the avenue fix has passed it. All comparison checks remain enabled.

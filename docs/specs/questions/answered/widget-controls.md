# Standard window controls: answered questions

## WC1 — ListBox residual adapters, rendering and malformed inputs

In sections 5.1–5.5, settle folded aliases, suppression nesting, clear/removal,
comparisons, style/gutter/bar setters, delivery ABI, malformed removal,
allocation and destruction, color/ellipsis/intensity and bar capture.

**ANSWERED 2026-10-24 (re-widget-controls-wc7):** DIFFERS — Section 5.6
and vectors L27–L36 specify all residual slots. Suppression was a mistaken
interpretation of sorting adapters. Some negative removals decrement count
without removing storage. NaN intensity survives, fractional intensity draws
no fill, short color prefixes erase temporary text and failed scans reuse
uninitialized/prior RGB words. Font fitting receives the original prefix;
the owner appends no ellipsis. Move and left-up use opposite capture tests.
Shutdown and destruction have separate resource domains; allocation failure
has no graceful owner return or rollback contract.

## WC4 — ListBox removal, drop sizing and next page

Filed by core-widgets5 on 2026-10-23: does removing the selected row retain
the index, how does origin adjust, and what are drop sizing and next-page rules?

**ANSWERED 2026-10-24 (re-widget-controls-wc7):** DIFFERS — Section 5.3 now
specifies that a selected row with a successor retains its index; only a
selected last row or removal before selection decrements it. Origin is
decremented only for removal before it, then clamped. Enabled drop height and
width policies run on insertion/replacement/removal and Init. Height clips
to parent bottom minus two pixels and whole lines; width measures raw stored
strings. NextPage requests first+wholeRows−1 then selects requested+wholeRows,
which can move origin again. Vectors L19–L25.

## WC5 — ListBox color escape recognition

Filed by core-widgets5 on 2026-10-23: which leading bytes introduce the color escape?

**ANSWERED 2026-10-24 (re-widget-controls-wc7):** MATCHES — The provisional
plain-text boundary can be extended: section 5.5 specifies ASCII `#` as the
introducer and `##` as a literal escape that removes one byte. A color prefix
uses seven bytes; recognition does not validate its hex suffix. Stored text
is unchanged. Vector L26. Malformed scan results remain in WC1.

## WC6 — TreeView root visibility and geometry

Filed by core-widgets5 on 2026-10-23: which flags include/hide the root,
what are indentation, viewport-count, image-enable and hit-rectangle rules,
and which selection entry anchors navigation?

**ANSWERED 2026-10-24 (re-widget-controls-wc7):** DIFFERS — Sections 6.3 and
6.5 now specify flag `0x2` root inclusion, `0x4` images, `0x8` connectors,
depth/rectangle arithmetic and viewport-derived floor/ceil row counts.
Navigation anchors on the last selected entry, not the first. Hidden-root
children are traversed even when that root is collapsed. Inclusive hit edges
and unsigned image-centering underflow are preserved quirks. Vectors T13–T18.

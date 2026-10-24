# Standard window controls: answered questions

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

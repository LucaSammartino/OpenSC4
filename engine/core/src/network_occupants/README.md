# Placement demolition helper

`placement_demolition::compute_demolition_region_for_placement_cells` implements
`docs/specs/network-routines.md` §§2–8: accumulate bounds and lot rectangles from
the input cells, publish a cleared region whose stored highs include the extra
clear border, scan the lot rectangles for zoned cells whose filtered occupant
lookup misses, reject on a `0xFC00` zone-type mask, and otherwise run the
demolition preview with the temporary `ExclusionFilter` (QI `0xA9764C8C`
complement) and return its result unchanged.

The host trait exposes only the individual foreign primitives (lot manager, lot
lookup and bounds, zone byte, filtered first-occupant lookup, QI/release,
DemolishRegion). Regions use the shared `zones::CellRegion`; city sides
64/128/256 give region sides of at most 257.

`RegionOutput` keeps any displaced region instead of dropping it, matching the
spec's "the helper does not destroy the previous value".

Not covered: the native region/filter memory layout, allocation-failure branches
(including the null-filter preview), malformed or out-of-city input (reported
as `Unsupported`, which is this crate's validation policy, not native
behaviour; see NRT1). Nothing is wired into the C ABI or the plugin; the
`network_rules` callers keep their existing foreign bridge.

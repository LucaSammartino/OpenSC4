# Network tool UI questions

## NUI1 — Numeric helpers and extra-anchor geometry

**2026-10-23 (re-network-tool-ui4), open.** Spec §§7.3–7.4, 9: finish the
ordinary finite-domain rule for LiesOnSegment `0x00645AC0`, local direction
`0x00645090/5130`, and LastDragDirection `0x00645BE0`. The latter's nonzero
path stores a float32 result then masks its raw bits with 7 (`0x00645C7F..C87`),
not an observed float-to-int conversion. What exact constants/operation order
and supported x87 control-state assumptions explain it? Publish literal
asymmetric/negative/collinear/endpoint cases before claiming the +2A nonzero
move outcome or an ordinary octant interpretation. Also trace every setter of
+2A; a press explicitly clears it. No unsafe native execution is requested.

## NUI2 — Bridge dialog enumeration, layout and indicators

**2026-10-23 (re-network-tool-ui4), open.** Spec §§7.2, 8–9: finish the UI-owned
AddBridgeEntry `0x00646260`, candidate ordering/filtering in release
`0x00646E40..F30`, indicator height predicate `0x006459A5..9CF`, and window
layout controls. Preserve raw primitive failure branches and reference cleanup;
do not substitute a foreign dialog/preview “success” oracle. The already-proved
window-creation-false release path leaves active=1/move-enabled=0 and does not
abort or decrease the lock. Which normal later inputs close that state? What
are the observed low-byte-only control arguments' actual foreign signatures?

## NUI3 — Secondary interfaces, flags and return-channel completion

**2026-10-23 (re-network-tool-ui4), open.** Spec §§3, 5, 9: complete secondary
table/ref/deleting-wrapper signature audit; distinguish inherited/shared stubs
from owner bodies rather than assigning all 41 catalog entries. Resolve actual
foreign bool widths for context result sound, neighbor flag and control-state
calls whose upper argument bytes are not normalized in the owner. DoPlacement
does not manufacture a canonical success return; which external callers, if
any, inspect residual AL/EAX? Keep raw GetFailure low-byte predicates explicit
even when the returned full word is nonzero. No ABI row is proposed in this lane.

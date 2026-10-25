# Standard window controls: open questions

Existing Grid, TextEdit and FileBrowser questions stay with their owner specs.
Target: Windows GOG 1.1.641. Filed 2026-10-23 by re-widgets-wg5.

WC1–WC6 are archived in
[answered/widget-controls.md](answered/widget-controls.md). Sections 5.6,
6.5–6.7 of the spec contain the settled owner behavior and conformance vectors.

## WC7 — TreeView RecalculateAll buffer and scrollbar policy

Section 6.5 requires RecalculateAll to measure content and update buffers,
scrollbar requirements and viewport. WC6 specifies content width, viewport
subtraction/capping and row counts, but does not specify the requested working
buffer dimensions, coupled horizontal/vertical bar thresholds, bar ranges,
position correction, or the ordering of resize/create/remove/place operations.
What are those observable policies (including zero content and equality)?
The core provisionally requests measured width by total*rowHeight and solves
content overflow on both axes before updating resources; that literal reading
is tested but cannot establish exact owner closure for section 6.5.


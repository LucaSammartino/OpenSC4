| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-viewutil-apply12 (VU15/VU16 shadow contracts) | `glue-viewutil-apply12` / (engine lane) | in progress | Apply contracts, gate, measure city to 2032. |

## Handoff
Goal: apply VU15/VU16 native contracts and measure all view utility shadow families.
Done: merged shadow11 at detached HEAD; added VU15/VU16 adapters for 36/50 bodies (rotation, 17 local helpers, 18 cached-input R-class operations).
Done: native PE identity and executable-section checks; all calls use local storage; scalar observation is stored float32.
Done: VU17 asks for complete private mesh resource offsets; VU18 asks for draw-cache field offsets. Existing VU12/VU13 remain open.
Done: implementation checkpoint 9189e7185; explicit owned-file fmt passes; rotation now has its own family summary and aggregate fields include all comparisons.
Next: collect focused.py cargo operation (session 69742, log build-runs/viewutil-focused.log), then run qbuild all and queued verify on stable source, then city to 2032.
Facts: focused build started before the final triangle/iterator additions; final gate must use stable current source. No game request outstanding. No public C ABI changes.
Facts: core cached adapter deliberately tests arithmetic using native cached inputs, not camera/frustum construction. 14 bodies remain unmeasured; default-off key viewutil_shadow.
Facts: unmeasured bodies: constructor, draw-box, city-clipped-box, camera update, viewport ray/ground/corners, mesh/model pick, visible area, draw build, iterator delete/select/next. No whole-owner verification claim.

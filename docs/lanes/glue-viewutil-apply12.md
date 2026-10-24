| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-viewutil-apply12 (VU15/VU16 shadow contracts) | `glue-viewutil-apply12` / (engine lane) | in progress | Apply contracts, gate, measure city to 2032. |

## Handoff
Goal: apply VU15/VU16 native contracts and measure all view utility shadow families.
Done: merged shadow11 at detached HEAD; added VU15/VU16 adapters for 36/50 bodies (rotation, 17 local helpers, 18 cached-input R-class operations).
Done: native PE identity and executable-section checks; all calls use local storage; scalar observation is stored float32.
Done: VU17 asks for complete private mesh resource offsets; VU18 asks for draw-cache field offsets. Existing VU12/VU13 remain open.
Done: implementation checkpoint 9189e7185; explicit owned-file fmt passes; rotation now has its own family summary and aggregate fields include all comparisons.
Done: native verify PASS, build 263/263 edges, CTest 81/81, 755.8 s (build-runs/verify-1.log). Tree-changed notice reflects handoff-only commit 0ca77e027; engine source unchanged since b6672b1de.
Done: turbo run 520, viewutil12-r1-turbo-20261024a clean to 2032-01-02, ref_warnings=0; 33/36 exercised bodies each 73/73 exact, aggregate 0/73. Three differing families map to VU12 and new VU19; 14 bodies unmeasured.
Next: collect focused.py (session 69742), qbuild all (session 94098, build-runs/all-1.log), old turbo event wait (42835, actual verdict already complete and valid), and normal game request viewutil12-r1-normal-20261024b.
Facts: focused build started before the final triangle/iterator additions; final gate must use stable current source. No game request outstanding. No public C ABI changes.
Facts: core cached adapter deliberately tests arithmetic using native cached inputs, not camera/frustum construction. 14 bodies remain unmeasured; default-off key viewutil_shadow.
Facts: unmeasured bodies: constructor, draw-box, city-clipped-box, camera update, viewport ray/ground/corners, mesh/model pick, visible area, draw build, iterator delete/select/next. No whole-owner verification claim.
Facts: native job qbuild-20261024-163355-build-ctest completed PASS. Source stable at b6672b1de; no implementation expansion after the 105070/150000 context checkpoint.

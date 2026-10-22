# View input controls: production core ownership

Published source: `docs/specs/view-input-controls.md` sections 1-8/10, and only
the supporting capture/cursor/end/sound facts of city-tools-input.md section 2.
No command maker, full base class, capped terrain owner, player-drive, flora,
torch, zone or network tool is owned here. No C ABI or native replacement.

`fixtures.md` records independent expectations before implementation. Tests use
literal expected state/effect traces and real core Terrain/ViewUtilities objects.
No original tool method, filter, snapping result, edit algorithm, selected area,
scroll vector or sign-position answer is a host callback.

## Published method inventory

This is the complete **published** method inventory for this scope. Entry
addresses below are spec identifiers only; no executable was consulted. **Every
method's byte length/end is unpublished**, so byte totals/coverage and complete
binary-owner inventory remain unknown (VIC5). Rust methods are semantic APIs,
not representations of native objects or their vtables.

| Owner | Published entry -> core implementation | Residual |
|---|---|---|
| CityScroll | construct 004B7DC0 -> Default; ReadTunableValues 004B8220 -> ScrollTunables consumed by init | Exemplar lookup is raw resource data; property/key constants exported in scroll.rs |
| CityScroll | Init 004B82A0 -> init; SetMouseScroll 004B82F0 -> set_mouse_scroll | Base Init/lifetime unclaimed; picture/resource failure VIC6 |
| CityScroll | IsSelfScrollingView -> is_self_scrolling_view; OnMouseMove 004B7EF0 -> mouse_move | Physical right/swapped-left state supplied by input adapter; no view right-press dispatcher |
| CityScroll | OnMouseUpR -> mouse_up_right; EndInput 004B7E70 -> end_input; Shutdown 004B7E00 -> shutdown | Release only capture taken here; no native base shutdown |
| LevelTerrain | construct 004BDC60 -> new; Init 004BDC90 -> new + init | Uses current Terrain cell-width/dimensions/altitude/rectangle API; no held native references |
| LevelTerrain | press 004BDCE0 -> mouse_down; move 004BDE30 -> mouse_move; release 004BDD80 -> mouse_up; keys -> key_down | Invalid vertex ranges Unsupported; shutdown-during-drag not specified (VIC6) |
| LevelTerrain | ComputeAndMarkSelectedArea 004BDB00 -> compute_and_mark; LevelTerrain 004BDB50 -> level | Inclusive cells and vertex extension owned here; no terrain brush implementation |
| LabelTool | construct 004BD230 -> Default; Init 004BC980 -> init | Local identity/cursor/show work only; command maker VIC1 |
| LabelTool | GetDecalOccupantAt 004BCDC0 -> decal_at; selection 004BCE80 -> select | LabelFilter is core-owned predicate; same-selection policy VIC6 |
| LabelTool | press 004BD890 -> mouse_down; move 004BD5A0 -> mouse_move; release 004BDA40 -> mouse_up | New/selected cancel paths separately owned; offset kept but unused |
| LabelTool | UpdateTextDecal 004BD320 -> update_text_decal + label_placement | Raw water/cell/network tile/edge data only; missing road tile Unsupported |
| LabelTool | FindRoadCenterAndOrientation 004BCB00 -> road_center_and_orientation; FindAvenueCenterAndOrientation 004BCEE0 -> avenue_center_and_orientation | All published ordered branches; unlisted numeric/edge inputs VIC3 |
| LabelTool | keys 004BD4B0 -> key_down; DoEditLabelUI 004BD6B0 -> edit_label; DoMessage 004BCA10 -> label_message_filter; Shutdown 004BD2E0 -> shutdown | Host modal loop invokes core filter; host never supplies dialog success policy; resource failures VIC6 |
| SignTool | Init 004D7BE0 -> new(code_page) + init | **Partial Init**: identity/cursor/show only. No service acquisition or notification binding (VIC2) |
| SignTool | selection 004D7FC0 -> select; press 004D81D0 -> mouse_down; move 004D84B0 -> mouse_move; release 004D8730 -> mouse_up | Core screen projection reused; pixel rounding VIC3; typing-release ambiguity VIC7 |
| SignTool | OnCharacter 004D8830 -> on_character; AppendText 004D8670 -> append_text; OnKeyDown 004D89D0 -> key_down | Glyph editing owned here, codec is primitive segmentation; invalid encoding Unsupported; unbound paste service Unsupported (VIC2/4) |
| SignTool | DoMessage 004D7D70 -> position_message | Core owns midpoint/projection/not-consumed reply; no unknown native payload encoding |
| SignTool | QueryInterface 004D7AF0 -> MESSAGE_TARGET_INTERFACE constant only; Shutdown 004D8130 -> shutdown | **QI unsupported** (no native object); **partial shutdown** (selection/edit cleanup only, no unacquired service or unregistered notification) |
| SignLabelDemolish | Init 004D7710 -> init; PickSignOrLabel 004D7820 -> pick; selection 004D7950 -> select; hover 004D7AC0 -> mouse_move; release 004D77C0 -> mouse_up | Renderer-first/fallback and highlight/remove/sound algorithms owned here; no unspecified destructor or shutdown override |
| LabelFilter | 004BC9B0 -> PropFilter::Label.accepts | Prop plus text-decal interface; no COM lifetime |
| SignFilter | 004D7CC0 -> PropFilter::Sign.accepts | Prop plus signpost interface; no COM lifetime |
| SignOrLabelFilter | 004D7680 -> PropFilter::SignOrLabel.accepts | Prop plus exact type IDs, independent of interface flags |

## Published state byte inventory

Offsets are **native spec facts**, not Rust layout or native storage ownership.
Where a width is not published, no width is inferred from the next field.

| Owner | Published byte/field facts | Core state |
|---|---|---|
| Base (support only) | +04 bool initialized; +08 u32 refcount; +0C u32 id; +10 u32 cursor id; +14 cursor*, +18 window*, +1C view*, +20 WM* | Only scroll local initialized/cursor and each tool identity constants; no refcount/raw pointers/full base |
| CityScroll | +28/+2C anchor; +30/+34 stored axis speeds; +38 has-scrolled byte; +39 prior-capture byte; +40/+44 inner/outer thresholds | anchor, stored_speed, has_scrolled, was_capturing, inner/outer; picture token tracked without native offset claim |
| LevelTerrain | own-field offsets/widths not published | width, dragging, start/end cells, previous cell-cursor setting |
| LabelTool | +28 message interface; +2C state 0/1/2; +34 press offset; +40 orientation | filter function; MouseState; press_offset; orientation; selected token (offset unpublished) |
| SignTool | +28 message target; +2C state 0/1/2/3; +34/+38 projected offset; +50 backup; +64 code page | position_message; MouseState; offset; backup; code_page; edit/pending/selection offsets unpublished |
| Demolish/three filters | no object-field offsets or sizes published | selected token and stateless typed predicates |

## Supported boundaries and exclusions

ControlHost is only cursor lookup/application (unknown resource preserves id),
capture/release, top removal(false), and sound-event posting. COM Init/Shutdown,
QI/AddRef/Release, cursor pointer ownership, window assignment, base constructor,
stack dispatch, and all other base slots are **not implemented or claimed**.
Native picture/window tokens must be acquired/released by an eventual adapter
under the documented ownership rules; this module neither holds nor releases
game pointers. Shared getters use `&self`, never RefCell.

The view's SetScrolling command is an external typed view operation, not camera
integration/timing. An adapter can call the existing CameraControl movement API
at its view cadence; this module does not invent the missing view update rule.
Label view_rotation is a primitive camera rotation query (CameraControl::rotation).
World queries are primitive renderer picking, prop attributes, network edge
bytes, cell conversion, terrain height and sea level; GUI operations are script
load/field access/modal execution/focus/shutdown, never DoEditLabelUI.

VIC1 command makers/global-terrain other cases and VIC2 framework service and
native message bindings remain unsupported. VIC3-7 record explicit numeric,
text, inventory, failure/lifecycle and typing-release residuals. LevelTerrain
is a tool owner, **not the capped terrain owner**. Player-drive/flora/torch/zone/
network tools are excluded. No app UI invocation, game file writes, C ABI,
native-exact/shadow/replaced/full-owner claims, or registration/default changes.

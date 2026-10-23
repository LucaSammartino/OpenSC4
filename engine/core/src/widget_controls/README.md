# Widget control owners

Source: `docs/specs/widget-controls.md` and its incorporated owner specs.
This module extends the existing window/token and legacy property-interpreter
layers. It does not create OS windows or wire the plugin/C ABI. Foreign traits
expose individual reference, resource, timer, scrollbar, message and drawing
operations; control state and notification decisions live here.

Work in progress: ListBox and TreeView have state/message models and all of the
family vectors WC-L01–L15, WC-T01–T12 and WC-F01. Those tests use recording
foreign services and establish no native-game or pixel comparison. Grid,
TextEdit and FileBrowser incorporated contracts have not yet been implemented.
No owner-wide closure is claimed.

`TreeView::insert_at` is a semantic ordered-child-list mutation used to build
fixtures. It deliberately does not map native ordering sentinels or sibling
references; native `insert` reports WC2 until that mapping is specified.
Tree flattening literally includes root; WC6 asks which flags alter that rule.
ListBox `paint_plain_text` covers already-fitting ordinary text; `paint` reports
WC1/WC5 when a font would require unknown escape/ellipsis preprocessing.
Tree `draw_item` covers resolved rectangles and the no-image/text case; full
paint reports WC3 after balancing guarded buffer access. No guessed pixel
algorithm is presented as compatibility behavior.

## Owner closure

This is a progress table, not a completion claim. Section numbers are from
widget-controls unless another spec is explicitly named.

| Owner / spec section | Code item | Test or remaining work |
|---|---|---|
| All / 1 shared boundary | `Foreign`, `Handle`, `replace_ref` | `wc_l01_query_views_and_null_retention`, `wc_l10_l13_color_and_reference_quirks` |
| Grid / 2.1 identity/inventory | Not implemented | Next implementation round; read win-grid §§1–6 and appendix A |
| Grid / 2.2 state/dispatch | Not implemented | Next implementation round |
| Grid / 2.3 mutation/paint/failures | Not implemented | Next implementation round |
| TextEdit / 3.1 identity/inventory | Not implemented | Next implementation round; includes text-edit, layout, encoding and adapters |
| TextEdit / 3.2 state/input/output | Not implemented | Next implementation round |
| TextEdit / 3.3 failures | Not implemented | Next implementation round; preserve TE5/TE8 |
| FileBrowser / 4.1 identity/defaults | Not implemented | Next implementation round; read win-filebrowser §§1–9 and appendix A.1 |
| FileBrowser / 4.2 dispatch/drawing/IO | Not implemented | Next implementation round |
| ListBox / 5.1 identity/inventory | `query_interface`, slot-count constants, `remove_storage`, `select` | `wc_l01_query_views_and_null_retention`, `wc_f01_storage_removal_is_not_selection`; open: WC1 adapters |
| ListBox / 5.2 construction/lifetime | `new`, `init`, `shutdown`, `destroy`, resource setters | `wc_l10_l13_color_and_reference_quirks`; Init/default/lifetime ordering needs more tests; open: WC1/WC4 drop sizing |
| ListBox / 5.3 entries/selection/geometry | insertion/replacement/removal, selection, metrics, hit test, visibility, bars | `wc_l02_l06_selection_indices_ids_and_insertions`, `wc_l07_l09_signed_metrics_hit_geometry`; open: WC1/WC4 removal/drop/page details |
| ListBox / 5.4 input/notifications | key, mouse and emitter methods | `wc_l14_l15_delivery_and_key_protocols`, `list_mouse_timer_strictness_and_no_font_background`; open: WC1 drop-bar capture |
| ListBox / 5.5 drawing/failures | `paint`, `paint_plain_text`, indexed colors | `wc_l10_l13_color_and_reference_quirks`, `list_mouse_timer_strictness_and_no_font_background`; open: WC1/WC5 text preprocessing/fractional fill |
| TreeView / 6.1 identity/inventory | `query_interface`, slot constants | `wc_t01_t03_identity_initialization_and_expansion`; open: WC2 adapters |
| TreeView / 6.2 state/lifetime | `new`, `internal_init`, `create_root`, `init`, `shutdown` | `wc_t01_t03_identity_initialization_and_expansion`, `tree_font_growth_shutdown_and_open_ordering`; open: WC2 restart, WC3 image init |
| TreeView / 6.3 nodes/selection | `insert`, `insert_at`, `set_descriptor`, `set_font`, expansion/selection/visibility | `wc_t04_t06_selection_and_ancestor_visibility`, `tree_font_growth_shutdown_and_open_ordering`; open: WC2 native ordering and descriptor ABI, WC6 root visibility |
| TreeView / 6.4 input/dispatch | `do_message`, `wheel`, `key_down/up`, expander `mouse_down` | `wc_t07_t09_wheel_sign_zero_and_missing_bar`, `wc_t10_activation_sequence_and_branch_toggle`, `wc_t11_bar_messages_always_delegate_to_base`; open: WC3 page/modifier/drag, WC6 navigation anchor |
| TreeView / 6.5 drawing | `recalculate`, `draw_item`, guarded `paint` | `wc_t12_missing_image_does_not_suppress_text`; open: WC3/WC6 layout/gradient/image failures |
| ListBox / 7 family vectors | tests WC-L01–L15, WC-F01 | All represented in named tests above |
| TreeView / 7 family vectors | tests WC-T01–T12 | All represented in named tests above |
| Grid/TextEdit/FileBrowser / 7 incorporated vectors | Not implemented | Next implementation rounds |
| All / 8 unresolved contracts | `Unsupported::Open` with question IDs | Known-domain tests plus explicit open returns; owner completion still pending |

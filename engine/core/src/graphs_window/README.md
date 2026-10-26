# City graphs window procedure

`Owner` implements `cSC4GraphsViewWinProc` from `docs/specs/graphs-window.md`.
Resource properties enter as raw `Definition` fields, rather than a completed
graph supplied by a host. Window, widget, drawing, notification and simulator
operations are foreign primitives. No C ABI or plugin wiring is added.

| Spec section | Code item | Worked case / explicit boundary |
|---|---|---|
| 1 Identity and lifetime | `Owner::new`, `init`, `shutdown`, `destroy` | `section1_constructor_init_repeat_shutdown_and_destructor_reuse_base_windows`; counted windows and generator QIs |
| 2 Data | `Definition`, `Series`, `TIMES` | `gw_c1_dates_and_all_time_scales_have_one_extra_sample`; `section4_line_setup_legend_capture_and_pie_without_population` |
| 3 Showing, hiding, buttons | display methods, `window_message`, `message`, `set_time_scale` | `gw_c5_second_button_range_selects_low_byte_and_returns_false`; SelectOption remains open GW3 at `Host::select_button` |
| 4 ShowGraph | `show_graph`, `setup_bar`, `setup_line`, `read_series` | `section4_line_setup_legend_capture_and_pie_without_population`, `section5_bar_vectors_only_fill_their_group_and_make_unused_date`; raw widget calls open GW1, unnamed line-only control open GW4 |
| 5 RefreshGraph | `refresh`, `refresh_bar`, `refresh_line`, `sample_dates`, `line_value`, `bar_value`, `axis` | GW-C1–C4 tests; `section5_start_date_strict_test_missing_history_and_contiguous_visible_lines`; axis-title index semantics open GW2 |
| 6 Widget slots | `Host::graph`, `Space`, `Arg`, `Host::axis_title` | `open_graph_contracts_are_explicit_unsupported_boundaries`; GW1/GW2 remain open, no graph renderer is invented |
| 7 Helpers | `SeriesRecords`, owned strings/map, `LineStyle`, `antialiased_line`, `line_query` | `section7_vector_growth_copy_strings_map_default_and_line_forwarding`; underlying cGZLineGraph construction/destruction is foreign GW1 |
| 8 Quirks and cases | tests `gw_c1`–`gw_c5`, bar/line/pie paths | all five worked cases are portable tests; no native execution claim |
| 9 Function inventory | portable methods and owned-container equivalents above | tests above; shared/foreign graph widget bodies receive no implementation credit here |
| 2.1 Graph definitions | `Definition`, `read_series` | `section4_line_setup_legend_capture_and_pie_without_population`, bar vector case |
| 2.2 Time scales | `TIMES`, `set_time_scale`, `sample_dates` | `gw_c1_dates_and_all_time_scales_have_one_extra_sample` |
| 4.1 Bar setup | `setup_bar`, `read_axis` | `section5_bar_vectors_only_fill_their_group_and_make_unused_date`; raw B/G slots remain open GW1 |
| 4.2 Line setup | `setup_line`, `LineStyle` | `section4_line_setup_legend_capture_and_pie_without_population`; raw G/L slots remain open GW1 |
| 5.1 Bar refresh | `refresh_bar`, `bar_value` | GW-C2 and group-vector case; raw B21/G34 remain open GW1 |
| 5.2 Line refresh | `refresh_line`, `sample_dates`, `line_value` | GW-C1/GW-C4 and strict-start-date case; G17 interpretation remains open GW2 |
| 5.3 Y axis | `axis`, `apply_axis` | `gw_c3_axis_range_250_and_fractional_and_explicit_step`; raw output slots remain open GW1 |

GW1–GW4 are open in `docs/specs/questions/graphs-window.md`. The trait defaults
return `Unsupported::Open` naming each question. Tests supply literal raw slot
responses to verify caller ordering, without claiming settled widget semantics.
No new questions were required. Invalid dates, axis ranges, required pointers,
and group indices stop as `Unsupported::Precondition`, rather than successful no-ops.

`Handle`, `Rect`, `Result` and `Unsupported` reuse `widget_controls`; handles are
the window layer's identities. The owner stores no duplicate window or control
state. The test adapter services window primitives with `ui_windows::base::Windows`
(tree lookup, parenting, flags, visibility, areas and notification targets).
Native adapters can use the same base/control owners. Created widgets and counted
QI outputs are released; borrowed lookup, font and parent handles are not.
The few unspecified precision/early-resource-cleanup choices have SPEC-GAP comments.

# Minimap owners

Portable `cSC4WinMiniMap` and nested `cTZMap`, from `docs/specs/minimap.md`.
Base windows use `ui_windows::base::Windows`; foreign reference, capture and
drawing primitives use `widget_controls::Foreign`. No C ABI or plugin wiring.

The section closure table will name each implemented section and its spec test.

| Spec section | Code item | Worked case / test |
|---|---|---|
| 2 Object layout | `MiniMap`, `new`, inherited window handle | `constructor_layout_and_inherited_query` |

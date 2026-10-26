# network-overlay-views: answered questions

<!-- from: core-overlay-views4 (2026-10-22) -->
- ~~**NOV2 - §1, SC4PathInfoView exact geometry/state:** publish numeric light-green/light-red RGBA,
  red-to-white pulse waveform/channel mapping, red-cross axes/alpha/altitude, line vertex format,
  other packed-state fields, path/cell/occupant order, coordinate space, and whether arrows/bars use
  capped 25 points or the full path. Are arrow lengths/directions in x-z or 3D? How are repeated/tiny
  first/end segments treated? Publish setters and lifecycle/defaults. [Core retains opaque key bytes,
  known blue/yellow/magenta with alpha 200, caps to 25, searches backward for last x-z segment >1mm,
  omits zero-segment bars, traverses rows with x varying first. Light/pulse colours fail with
  PathInfoColourNov2; native line format stays None. Cross provisional reading: x/z axes ±4m, alpha 200,
  same +0.5m height. No complete-owner credit.]~~

  **Native evidence 2026-10-22 (re-overlay-views-contracts4):** §§1.1–1.2 prove exact colours/pulse
  update, format/state/defaults, full-vector 3D arrow/bar rules, ±2m opaque cross, x-outer traversal,
  empty/repeated/nonfinite and scratch-alpha boundaries. Residual: pulse static initialization,
  actual public filter/depth setters and shared cleanup/refcount lifecycle. The constructor does not
  reset the process-wide pulse; no initial-draw pulse value is certified yet.
  **Qualification 2026-10-23 (re-overlay-views-contracts4):** overlay-views §3.4 proves shared
  QI/AddRef/Release and path-info deleting cleanup individually. NOV2 remains partial for the static
  pulse initializer and actual public filter/depth setters; geometry vectors remain UNEXECUTED.

  **ANSWERED 2026-10-26 (re-ovv29):** DIFFERS — network-overlay-views §§1.1–1.2 establish loader-zero
  pulse at 0x00B228C8, first Update=5 and shared period 52 Updates; depth uses folded byte setter
  0x007DC1A0 without repacking, filter is an inline full-u32 store with only 0xFFFFFFFF wildcard,
  and Shutdown is the shared flag-clear body 0x0053C940. Existing exact geometry/colour rules and
  overlay-views §3.4 supply the remaining contract. The worked state case in §1.2 is UNEXECUTED.

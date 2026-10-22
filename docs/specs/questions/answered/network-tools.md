# network-tools: answered questions

<!-- from: core-powerline4 (engine lane, 2026-10-21) -->
- **NT19 - network-geometry §11, parameter defaults:** defaults of +0x3A8/+0x3AC/+0x3B0 and meanings of
  +0x3A4/+0x3A8; core used zero for missing properties.
  **ANSWERED 2026-10-22 (re-powerline-completion4):** DIFFERS — network-geometry §11/power-line-tool §§1,4:
  fresh Init defaults uint32 20, float32 7.0, uint32 0, float32 0.0; spacing in drag steps at pivots;
  clearance raises ray endpoints. Re-Init resets only spacing/clearance. Missing cost properties retain
  prior values. Evidence `0x006506AA–0x00650704`, `0x00650A43–0x00650A62`, `0x00650CE8–0x00650D9C`.

- **NT20 - §9 regional pairing and §11 reserved walk:** does Place depend on Attempt, and how are trailing
  zone/road cells and classifier results above 3 treated?
  **ANSWERED 2026-10-22 (re-powerline-completion4):** DIFFERS — network-tools §9: Attempt runs in both modes;
  Place requires success and real=true (`0x00652A34–0x00652A46`); gate counts input anchors, not poles.
  Network-geometry §11/power-line-tool §6: zones/roads preserve last except final road after result 0,
  which stops at currentStep-1. Clear sets last; initial last=count-1 explains an all-zone walk retaining
  count. Results above 3 preserve last/failure but replace previous result; first previous is uninitialised.

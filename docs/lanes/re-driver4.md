| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-driver4: RE spec of the DX7 3D driver (nSGLDX7 cGDriver family) | `re-driver4` / (RE lane) | in progress | see Handoff |

## Handoff
- Goal: docs/specs/renderer-driver.md: nSGLDX7::cGDriver (iface vtable 0x00AC4660 at obj+0xC) lifecycle/state from exe bytes.
- Done: spec sections 1-5 (scope, construction, error channel, Init/Shutdown/SetVideoMode/3D env/BeginScene/Flush device-lost cycle/teardown, state slots, Enable caps, colour quantisation); questions RD1-RD5 filed.
- Next: Q-RD3 video modes (CountVideoModes 008885f0, GetVideoModeInfo 00886e10/00887680) if budget allows; then finish report.
- Facts: decomp frames differ: slots called via iface (obj+0xC) use iface offsets; error code obj+0x11c, mode idx obj+0x1e0, mode count obj+0x1d4, device obj+0x224.
- Other two 83-slot vtables: 0x00AC36D0 (OpenGL driver, inferred), 0x00ACC4D8 (third driver) -- not covered.
- Helpers: build-runs/fn.py (decomp body by addr), build-runs/rd.py (read exe data).

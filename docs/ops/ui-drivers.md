# Scenario UI drivers

The DLL accepts these opt-in dated `.script` actions in the test slot. UI
actions run at framework ticks, in file order, separated by at least 150 ms.
`uiwait` uses wall time, so a paused modal window can still receive input.
Actions following a UI action wait behind it. Modal loops can pump input,
but nested commands and simulator/save actions wait until the command returns.

| Verb | Arguments | Delivery |
|---|---|---|
| `uicommand` | `0x12345678` | Registered ID through command-server slot 20, with separate input/output sets. |
| `uimouse` | `x y move\|down\|up\|click` | Client pixel coordinates; posted mouse messages. |
| `uicell` | `x z height move\|down\|up\|click` | Cell centre `(16*x+8,height,16*z+8)` projected by view utilities, then converted to main-window pixels. Height is metres above datum. |
| `uicontrol` | `0x12345678` | Find a child by its resource ID and click its centre through ordinary mouse input. |
| `uikey` | `1..255` | Win32 virtual-key down/up, including scan and transition bits. |
| `uitext` | `printable ASCII text` | Up to 256 WM_CHAR characters, preserving internal spaces. No implicit Enter. |
| `uiwait` | `0..60000` | Delay subsequent actions by milliseconds; never sleeps the main thread. |

Mouse input targets only the current process's `GDriverWin`; it neither moves
the desktop pointer nor changes another slot's focus. Points outside the client
are refused. Use separate down/move/up actions for road drags. Projection is
restricted to the identified supported game image and a live city view.
Unknown command IDs are refused. `status=0` and `posted` prove delivery only:
qualification requires target-method reach and a clean verdict with no reference
warnings. `ui_command_catalog=1` logs registered IDs/names/parameters on the first
command for diagnosis; it does not execute the catalogue.

The `ui29-candidates` suite group contains house-city, West Weburg and Bevopolis
censuses. These are qualification candidates, excluded from `full` until clean
game results and per-owner reach establish the actions they actually exercise.
Map layers, individual sub-budgets, tree controls, web/HTTP, nation and volcano
paths still need qualified scenarios. No coverage credit is claimed for them.

The installed accelerator resource `{A2E3D533,6A231EAA,6A9362EF}` supplies query
`6a935cf3`, road `6a935dc0`, MySim `6a935ca8`, mayor `6a935ca4`, play/graphics/audio
options `6a935e2a/6a935e2d/6a935e34`, and close `6a935cf1`.
The general accelerator resource with instance `6A9362F1` supplies cheat dialog
`2a9496c9`. Installed UI resources contain budget button `00000041` and news
button `ca2aedcc`. Resource contents and extracted game data stay uncommitted.

`reach_startup=1` arms tracing immediately after the director's entry snapshot,
before subsystem registration and application init. Its sentinel date is
1999-01-01; `reach_lifecycle=1` opens the 2000-01-01 load window, followed by the
saved city's real date. This cannot capture anything before the DLL director
is constructed. Use a plain run to distinguish natural startup from setup probes.

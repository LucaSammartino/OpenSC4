# mysim-windows: open questions
<!-- Questions about [mysim-windows.md](../mysim-windows.md). Append new entries at the END; when one is answered, move it to answered/mysim-windows.md. Rules: README.md. -->

<!-- from: re-mysimui (2026-10-21) -->
- **MW1 - §1.4, §1.5:** what the set-aside list (`+0x38`) and item flags 2 and `0x20` mean for the shown
  message list, given that `AddMessage` takes items out of it and `RemoveMessage` puts them in. [as read]

<!-- from: re-mysimui (2026-10-21) -->
- **MW2 - §1.6:** what the window does with `0xAC08746E` and `0xAC08746F`, and the layout of the statistics
  captions. [as read]

<!-- from: impl-mysim30 (2026-10-26) -->
- **MW3 - §1.3, §1.5:** which transport-button images correspond to which Sim state, and what are the exact
  advice kind/flag-to-colour/icon mappings, HTML structure and title/note selection rules? The summary gives
  palettes, fonts and an icon range but not these decisions. The core reports Unsupported for image selection
  and HTML construction, while retaining the documented raw list operations and redraw requests.
  Also, how are the five vehicle icon properties assigned to the transport dialog's widgets?

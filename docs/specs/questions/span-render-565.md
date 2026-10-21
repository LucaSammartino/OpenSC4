# span-render-565: open questions
<!-- Questions about [span-render-565.md](../span-render-565.md). Append new entries at the END; when one is answered, move it to answered/span-render-565.md. Rules: README.md. -->

<!-- from: re-span565-4 (2026-10-21) -->
- **SR565-1 - section 4.1 / 4.2:** does any shipped code path draw a gradient (`+0x74` set) into a colour-type 5
  buffer? `SetGradientTransform` (`0x0082FDB0`) and its base forwarder `0x0082C360` have no direct caller in the
  export. If a path exists, an in-game capture would show whether the B-channel drift of section 4.2 (start shifted
  right by 3, step with its low 3 bits dropped) is visible.

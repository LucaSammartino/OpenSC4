# network-manager: open questions
<!-- Questions about [network-manager.md](../network-manager.md). Append new entries at the END; when one is answered, move it to answered/network-manager.md. Rules: README.md. -->

<!-- from: automata-actions29 (2026-10-26) -->
- **NM20 - section 9; native consumer after generated s2 network actions:** Please identify the
  consumer/interface boundary at `SimCity 4.exe+0x3529C0` (`0x007529C0`), its required object view,
  nullability and ownership contract, and the subsystem that supplies that object. With the fixed
  NM17/AT25 automata boundary, the full house list's `gen27-konradshohe-s2-grow.script` stops at
  **84/216** actions: `aa29-at25-house-s2-b.log` reads null+`0x6C` on 2026-09-01. Removing only
  `automata` produces the **same fault and 84/216** on 2026-08-31 in `aa29-at25-s2-noauto-b.log`;
  that log explicitly confirms automata is off. Both runs use DLL SHA256 prefix `afb59058`.
  Logs and exact requests are in main's `build-runs/game-results/` and `build-runs/game-queue/done/`.
  The last scripted call is line 88, `network 40 51 40 19 5`, returning `ok=0`, followed by tool
  Shutdown. This timing does not establish that placement caused the fault: please route the
  boundary to another spec if necessary. Section 9 specifies the placement sequence but does not
  identify this later consumer. A full-house repeat on the earlier same-source DLL (`6c4bb917`,
  `aa29-at25-house-s2-a.log`) has the same fault at 84/216. In contrast, s1 completes 216/216 and
  quits cleanly (`aa29-at25-house-s1-a.log`). [current choice: leave s2 unaccepted; stop this item
  rather than invent a layout, null fallback or native ownership rule]

# mysims: open questions
<!-- Questions about [mysims.md](../mysims.md). Append new entries at the END; when one is answered, move it to answered/mysims.md. Rules: README.md. -->

<!-- from: re-mysim-dispatch5 (2026-10-23); residual of MS17 -->
- **MS18 — multimodal route-ready handoffs (§4.6.1, §4.11, §7.2.1, §8):** complete the dispatch state-2 route-ready automaton in `switchTravel` (`0x00435840`): mode-to-mode handoffs (0/1/2/3/6/7/8/0xFE/0xFF), stop selection and neighbour retries, final-segment proximity geometry/rounding and equality boundaries, idle versus stuck recovery, full notice ordering including `0xEBA06BD0`, `0xEBA06BD4`, `0xEBA0732E`, `0x2BAC398F`, and any delegated automata/model RNG. Also close dispatch-tool input lifecycle and player-drive mission callbacks beyond the GoTo/Enter/Exit contracts now specified. MS17's queue, clock/wait timestamp, notification setters, five tuning field uses, request entry points and serialized payload are settled; the whole dispatch owner is **not** yet specified. [Do not infer visible motion, retry selection or complete-trip success from accepting a destination request.]


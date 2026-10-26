# mysims: open questions
<!-- Questions about [mysims.md](../mysims.md). Append new entries at the END; when one is answered, move it to answered/mysims.md. Rules: README.md. -->

MS18, MS20, MS21 and MS22 are answered in the archive and specified in
mysims.md §4.6.2–§4.6.5, §6.2.1, §7.2.2–§7.2.3 and §12.29–§12.30.

No open questions.

- **MS-GC29-1 — shadow scheduler state at capture (advisors §3.1, §5.2).**
  `growth-cities29-2/london-sydenham.log:2858` captures MySim IDs 1–5 with
  month counter 1 and current day 2451545, but ID 30 with 0/0. At :3155 the
  deferred PostCityInit increments every shadow counter; at :5429 all six
  differ by one month (ID 1 original 3, shadow 4). The system's `lastDay`
  is unsaved and the shadow only imports individual advisor records. What
  documented offset/interface and dispatch boundary expose the original's
  `lastDay` for a same-input shadow? Is PostCityInit already processed when
  these new-city records are captured? Freezing the establishment flag at
  capture did not remove the difference and was reverted. Blocks mysims in
  Sydenham, Woolwich, Terraforming and the three empty-city scenarios.

  Follow-up: `growth-cities29-5/reach29-growth.log:3067` and
  `growth-cities29-5/lua-lots-empty.log:3270` reproduce the six-sim mismatch
  at the first February month. The house and both mature reach scenarios
  remain exact. These newly measured new-city cases need the same scheduler
  input/load-boundary clarification; no monthly comparison was suppressed.

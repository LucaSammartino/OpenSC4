# sc-resources: open questions

<!-- Questions about [sc-resources.md](../sc-resources.md). Answered entries move to answered/sc-resources.md. -->

No open questions. SCR1–SCR5 and SCR7 were answered by re-scres-scr5;
SCR8/SCR9 were answered on 2026-10-24 by re-scres-scr8 in spec section 10.
SCR10 was answered on 2026-10-24 by re-scres-scr10 in spec section 11.
See [answered entries](answered/sc-resources.md). Runtime coverage remains
separate from these answered native contracts.

<!-- from: glue-scres-replay7, runs scres7-physical-2032-ash/birch (2026-10-24) -->
- **SCR11 — sections 4.2/11.2, physical file block-comment preprocessing:** What is the complete preprocessing grammar between ReadLine and include/ordinary-command routing? In a clean 2032 turbo run, Graphics Rules physical line 216 is `#<`; that line and subsequent physical lines (including empty lines, plain explanatory text and assignments) produce no command/include boundary. There are 45 such reads across the file, while ordinary blank/comment lines elsewhere do reach ParseCommand. All 1107 true reads increment physical/inclusive counts (root own=684, child own=423, root return=1107); false reads add zero. The strict literal line-comment replay differs 72 times after the first divergence, with no resyncs; the optional open-failure replay is exact. Please specify start/end markers and matching positions, whitespace/case, inline/same-line forms, nesting, whether includes or quote/group delimiters matter inside a block, EOF with an unclosed block, and whether state is local to each recursive file or shared. [Current choice: production core retains section-2 `#` line-comment semantics only; no block grammar is guessed. Full raw replay stays default-off and reports SCR11 differences. A separately labeled partial file-control replay takes observed preprocessing dispositions as an explicit foreign input, retaining every physical read, position, include/open and return count. Native ABI is not in question.]

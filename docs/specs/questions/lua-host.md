# lua-host: open questions
<!-- Append-only questions from clean-room implementer lanes. -->

No open entries. LH1–LH5 are in [answered/lua-host.md](answered/lua-host.md).

<!-- from: glue-lua-shadow5 (2026-10-24) -->
- **LH6 - sections 1.4, 1.5 and 13, complete record-only interception:** what supported installation point captures all existing app/advisor, temporary automata and ScriptServer Lua instances before their first Open/load/registration, including subsequent cISCLua/cIGZLua5Thread calls and Lua-internal table accesses? Section 1.4 explicitly establishes no safe callback installation recipe, and public retained interfaces do not expose a call observer. [Do not patch native dispatch or substitute services; measure the original extension collector and registration state through the documented collector layout and public Lua interface only. Full chunk/call/state replay remains disabled.]

<!-- from: glue-lua-shadow5 (2026-10-24) -->
- **LH7 - sections 3, 9 and 16, replay nondeterminism:** which Lua/library/foreign callback outputs may a full same-process replay exclude or inject as recorded inputs (time, random state, userdata identities, registry notifications and iteration order), and how are those values normalized? The spec does not enumerate a replay exclusion contract. [No nondeterminism exclusions in the bounded collector/registration comparison; no claim to full VM replay or full Lua verified coverage.]

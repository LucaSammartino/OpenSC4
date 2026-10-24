| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-scres-native6: natural resource parsers | Codex, `glue-scres-native6` (reserved branch, detached HEAD) | READY: natural command tokens exact 842/842; scres 76/76; full gate PASS | Publish SCR10 for full file replay; coordinator review coverage/profile credit for the measured boundary. |

## Implementation

`72c7ccde4`, corrected by `1962e01cd`, applies sc-resources section 10's shared
ParseCommand/ParseCommandArguments ABI. Default-off `scres_shadow` now observes
actual startup rule-file traffic, not just token adapter tables. Hardware entry
breakpoints cover shared direct calls without decoding/copying executable bytes
or assuming instruction spans. Observation is restricted to the installing game
thread; occupied debug registers refuse, and saved registers are restored on
stop. Borrowed strings are copied while alive; no borrowed interface is released.

Native argument vectors at dispatch entry are compared with production
`core/src/sc_resources/lexical::split`, the service used by CommandParser::parse.
This compares token count and bytes before substitution, including variables,
quotes, groups and comments. Registered resource-file paths, recursive file-call
depth, returned native line totals, exceptions, nondispatch lines and expanded/
standalone dispatch are counted separately. File-route token samples are
distinguished from other routes. Empty captures cannot report exactness.
Constructed adapter probes remain independent and unchanged. Public C ABI
exports and FC4_ABI_VERSION are unchanged.

## Validation

- Focused capi scres tests **5/5**; owned Rust-file fmt **PASS**. Initial relative
  fmt invocation used the wrong tool-relative path; absolute-path invocation and
  in-place layout corrections passed.
- Final corrected-source `qbuild all` **PASS**: workspace **6890 passed**, zero
  failed/ignored; clippy `-D warnings` zero diagnostics; native build **92/92**
  (4.4 s), CTest **72/72** (79.5 s), native total 85.4 s.
- Final queue request `qbuild-20261024-033751-build-ctest` collected to terminal.
  Logs: `build-runs/qbuild-{test,clippy}.log`,
  `build-runs/fc4verify-{build,ctest}.log`. No build operation outstanding.
- Earlier full gate also passed: 6890 tests, zero clippy diagnostics, native
  233/233, CTest 72/72; request `qbuild-20261024-032354-build-ctest`.
  The first game log exposed an inaccurate shutdown scope label; that concrete
  diagnostic led to the corrected summary, route counters and final repeated gate.

## Real-city evidence

Final run [scres6-native-2032-elm-401](../runs/scres6-native-2032-elm-401.md),
request label **scres6-native-2032-elm**, Konradshohe, turbo to **2032-01-05**,
**clean**, **ref_warnings=0**, all problem counts zero. Exact coverage summary:
**`scres 76/76`**. DLL SHA256
`4dbaf830a787d9ff1a5c63f65320b6c7cc593eb0fc39a42b3a68b83a00630fc6`.

| Observed boundary | Result |
|---|---|
| Native command tokens | 842/842 exact; file_calls=842, other_calls=0, refused=0 |
| Shared line entry | 1061 calls; 219 nondispatch; expanded/standalone=0 |
| Native file route | 2 registrations, 3 file calls; includes at depth 2 |
| Native file totals | 1530 inclusive line totals; 1 native exception boundary observed |
| Constructed binary adapters | 24/24 exact |
| Constructed text adapters | 22/22 exact |
| Shipped resource-prefix numeric adapters | 10/10 exact |
| Dated shadow lines | 73 natural lines + 3 constructed lines; all mismatches=0 |

The game actually registered Graphics Rules.sgr and optional Graphics Options.sgr;
the recursive call to Video Cards.sgr was captured. Inclusive file line totals
are native return values, not a unique physical-line count or replay equality.
No token adapter calls occurred naturally; no empty adapter equality line is
emitted. The native exception count records an observed boundary and does not
claim exception replay parity; harness foreign/structured exceptions were zero.

Earlier [scres6-native-2032-cobalt-400](../runs/scres6-native-2032-cobalt-400.md)
also reached 2032 clean with **scres 76/76**, ref_warnings=0, on its distinct DLL.
Both unique requests were collected by one event-driven verdict wait each.
No request was discarded or duplicated; no game request remains outstanding.
No mismatch occurred, so a normal-speed rerun was unnecessary.

## Open scope and coordinator follow-up

**SCR10**, appended to `docs/specs/questions/sc-resources.md`, requests the
missing cRZTextFile::ReadLine ABI and file association contract. Complete
physical file-parser replay (include directives, blank lines, EOF/read failure
and positions) remains unimplemented/default-off. This lane also makes no
macro/variable/custom-command state, complete INI resource, image or ASCII writer
parity claim. The measured natural boundary is command-line tokenization on
the actual startup resource-file route; constructed raw INI prefix reads still
do not establish INI parser coverage.

Ready to land the measured work under the task's partial-progress rule. The
coordinator should publish SCR10, review coverage credit for this boundary, and
append existing `scres_shadow` to the current single regression shadows string
if accepted. `scres_constructed` already belongs to the house profile. No tools/
file or coverage map was opened, edited or run, and no merge/ref update was made.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-scres-replay7: physical file replay | Codex, `glue-scres-replay7` (reserved branch, detached HEAD) | READY for measured partial scope: file control 73/73 exact; existing scres 76/76; full raw replay blocked on SCR11 | Answer SCR11, then complete raw replay; coordinator house regression and coverage review. |

## Implementation

`60758578d` applies SCR10's physical-line observation contract. The default-off
`scres_file_shadow` key extends the existing hardware observer with scoped
constructor/open/ReadLine capture, without reading executable bytes. Its third
debug-register probe is reused during each recursive file invocation; the
registration probe is restored afterwards. Borrowed native strings are copied
with their lengths before native comment processing. File identity, open bool,
physical count, logical position, true/false reads, includes, ordinary-command
boundaries, normal returns and unwind are kept separately. No borrowed pointer
is retained as an owned reference, acquired or released. ReadLine false is
explicitly EOF-or-failure; it is never mislabeled as a distinct read error.

The production core file loop now exposes a command-execution boundary and
returns inclusive counts. FileParser uses that same loop. The private static-link
transport replays the native event stream in order without seeking or resync.
Logical byte positions are captured and checked for monotonicity/association;
this is not an independent ReadLine byte-decoder differential. Mode-2 input is
refused. Public C ABI exports and FC4_ABI_VERSION are unchanged.

`81bb13d53` adds bounded diagnostics after the first real game mismatch.
`86831123c` retains strict raw replay and adds a separately labeled partial
file-control replay. The latter takes observed preprocessing decisions as an
explicit foreign input; it does not implement or validate a block-comment
grammar. The strict raw differential remains visible, with `question=SCR11`.
Both paths stay default-off. No key was added to a general or house profile.

## Final validation

- Focused capi scres tests **8/8**; owned Rust-file formatting **PASS**, zero diffs.
- Final stable-source `qbuild all` **PASS**: workspace **6905 passed**, zero failed
  or ignored; clippy `-D warnings` **0 diagnostics**; native build **194/194**
  (123.7 s), CTest **74/74** (166.6 s), native total **292.0 s**.
- Queue `qbuild-20261024-043532-build-ctest` was collected to its terminal PASS.
  Engine source stayed at `86831123c` throughout that gate and both final runs.
  Full logs: `build-runs/qbuild-{test,clippy}.log`,
  `build-runs/fc4verify-{build,ctest}.log`.
- Earlier gates passed on their distinct sources: Rust6904/native240/CTest74,
  request `qbuild-20261024-041758-build-ctest`; Rust6904/native47/CTest74,
  diagnostic request `qbuild-20261024-042645-build-ctest`. Each repeat followed
  a concrete game diagnostic and changed source. Their associated game evidence
  is retained, rather than relabeled as final-source evidence.

## Real-city results

All four unique requests reached 2032 in Konradshohe, verdict **clean**,
**ref_warnings=0**, every harness problem count zero, no resynced lines.
Each verdict was collected with one event-driven wait; no request was discarded.

| Run record / request label | Exact coverage summary | Scope |
|---|---|---|
| [403](../runs/scres7-physical-2032-ash-403.md) / `scres7-physical-2032-ash` | `scres 76/149` | First strict raw replay; 72 differences per dated raw line |
| [404](../runs/scres7-physical-2032-birch-404.md) / `scres7-physical-2032-birch` | `scres 76/149` | Diagnostic capture establishes the missing preprocessing boundary |
| [405](../runs/scres7-control-2032-cedar-405.md) / `scres7-control-2032-cedar` | **`scres 149/222`** | Existing76/76 plus partial file-control73/73 exact; strict raw0/73 under SCR11 |
| [406](../runs/scres7-default-2032-dogwood-406.md) / `scres7-default-2032-dogwood` | **`scres 76/76`** | Final DLL with new key omitted/default-off; existing profile exact |

Final DLL SHA256:
`7e1bdf3e0d8436ad9374d3f6475f8d41d5ee24285c91cb4070e16aa76380be43`.
Final runs reached 2032-01-04 / 2032-01-17, respectively. Their dated groups
were independently counted from the logs: control73 exact, raw73 differing,
and existing-default76 exact. This is the existing scres profile check, not a
full house-profile regression; the coordinator still needs the standard house
check. House and coverage tool files were not opened, edited or run.

The combined run observes 1107 true physical reads (122 empty), two terminal
false reads, three opens including one failure, 1061 ordinary command calls,
one recursive include, and 1530 inclusive native line totals. Graphics Rules
has 684 own lines; Video Cards has 423; the parent returns 1107. Optional
Graphics Options open failure replays exactly. Native command tokens remain
842/842 exact; constructed adapters remain 24/24 binary, 22/22 text, and 10/10
shipped resource prefix. No command-state, arbitrary exception, image, writer,
INI-parser or ReadLine byte-decoder parity is inferred.

Representative counted lines, repeated monthly through 2032:

```
scres shadow 2026-01-27 load: partial=file_control_replay foreign_preprocessing=45 reads=1107 false_eof_or_failure=2 includes=1 inclusive_totals=1530 resyncs=0; mismatches=0
scres shadow 2026-01-27 load: partial=raw_file_parser_replay question=SCR11 roots=2 replays=2 reads=1107 blank=122 false_eof_or_failure=2 opens=3 open_failed=1 commands=1061 includes=1 physical=1107 inclusive_totals=1530 undispatched=45 refused=0 resyncs=0; mismatches=72
```

## Open contract and next steps

**[SCR11](../specs/questions/sc-resources.md)** asks for physical block-comment
preprocessing grammar. The first observed suppressed input is physical line216,
`#<`; 45 reads have no command/include boundary, including blank and noncomment
text. The supplied spec defines line comments but not this grammar. The core
keeps its literal line-comment behavior, marked in code. Every raw discrepancy
is attributed to SCR11; no grammar was guessed. These captures precede city
turbo activation, so this is not a difference seen only under turbo.

Ready under the brief's measured-work fallback. **The full raw replay goal is
not met.** The new partial measurement concerns `nSCRes::cFileParser` section4.2,
but does **not** justify marking its whole coverage-map owner row verified.
No new whole-owner row is claimed. Existing command-token/adapter measurements
are preserved. Keep `scres_file_shadow` off and outside the house profile until
SCR11 is answered, implemented, and the complete raw replay is exact in 2032.
The coordinator should regenerate the questions index, run the standard house
regression, and review coverage/profile credit only after full equality.
No build or game request remains outstanding. No merge/ref update or push made.

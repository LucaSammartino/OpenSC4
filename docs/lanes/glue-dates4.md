| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-dates4: DT2/DT3 date adapter | `glue-dates4` / engine lane | in progress | Reconcile published contracts, gate, and independently verify native dates and our adapter in game. |

## Handoff
- Goal: reconcile DT2/DT3 core/CAPI/cFC4Date and verify original-versus-Rust from identical inputs in process.
- Worktree reserved, detached HEAD; no branch/ref changes. No RE, subagents, tools/maps/default edits.
- Done: core/CAPI milestone 2ca78537. Baseline all PASS: cargo4938/0, clippy0, CTest68/68 (tree changed during baseline; final stable gate still required).
- Done: adapter now uses Rust unsigned calendar, EAX day-number setters, Clone EAX1/ref1, native temporary Copy and real allocator cleanup EAX; atomic date state.
- Done: opt-in dates_probe via existing simulator GetSimDate; only published original cGZDate vptr qualifies. Live stateless comparisons separate from explicitly seeded owned-clone fixtures.
- Done: foreign Copy, native/foreign owning outputs, byte/NUL/high-byte/page-boundary fixtures, full EAX counters; DT4/DT5 filed for executable CRT locale/timezone acquisition.
- Checks: quick PASS cargo3453/0 and119/119 build edges; four touched Rust files fmt PASS. Earlier quick failures (return-type mocks, logging macro) corrected.
- Full gate first attempt: Rust4940/0, clippy0, CTest67/68; plugin_load_test expected old diagnostic Year text. Updated trace assertion and old Set(100) bool assertion to published EAX100.
- Added explicit refs7/incoming-EAX/source-release Clone fixture and complete SetTime/locale-unresolved counters before final stable gate.
- Next: stable qbuild all; freeze DLL, unique original/adapter/40-key game requests with scripted save and shutdown; preserve complete mismatches/refusals.
- Published B string cleanup entry is already supplied by compat::StlNodeAllocator; capture EAX after that call, never synthesize success.
- Executable CRT locale acquisition is not published; park precisely rather than use plugin CRT locale as evidence.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-dates4-r1-manifest.json.
- Game requests: none. At most one outstanding; unique immutable labels, wait without qwait.

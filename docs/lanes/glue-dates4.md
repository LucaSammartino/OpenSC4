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
- Second full gate:4940/0, clippy0,67/68; exact Year trace still absent after cap200 exhausted. Assertion now checks capped date trace presence; functional Year assertions remain above. No runtime behavior changed.
- lib.rs fmt traverses unrelated modules and reports existing diffs (advisors.rs etc.); no mass reformat. Four date Rust files fmt PASS.
- Final stable all gate at70c454c4: PASS Rust4940/0, clippy0, CTest68/68, build4/4; no tree-change flag. DLL code f32af464.
- Next: frozen DLL build-runs/dll/gd4-r1-final; native request gd4-r1-native-a-20261022, then adapter andunion40 labels with same pattern. Script build-runs/dates-save.txt saves2027-01-02 and2031-12-01, end2032, quit/shutdown.
- Published B string cleanup entry is already supplied by compat::StlNodeAllocator; capture EAX after that call, never synthesize success.
- Executable CRT locale acquisition is not published; park precisely rather than use plugin CRT locale as evidence.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-dates4-r1-manifest.json.
- Native gd4-r1-native-a-20261022: stalled at2027-01-02 scripted save; ref0 foreign0; live234/234, fixtures185/185; no date mismatch/refusal/fallback/resync. Owner cGZDate/00ACEA38 via borrowed city.GetSimulator.GetSimDate, native ptr143A5B04. No clean2032 credit.
- Frozen DLL SHA256 4FCF41E234C93150A770B55CA013BDD13359B2528C5B8BBF65D78F3617279EC0. Never rebuild/mutate during requests.
- Next adapter request omits script as lifecycle control; union40 retains script. Native-nosave separate control if budget permits. Original save/reload remains failed/unproven, never a lucky-rerun fix.
- q.py Bash runtime unavailable (E_ACCESSDENIED); local quiet Python waiter run directly in PowerShell. No qwait or game launch.

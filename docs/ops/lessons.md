# 5. Lessons that engine comments and lane prompts cite

For engine/plugin work. References to "HANDOFF §5.1" mean §5.1 here; keep these section numbers stable.

### 5.1 Threading
The game calls our getters from a second thread while the main thread is inside `OnTick`. Shared state that getters
read must not live in a `RefCell`. Details: foundations.md §19.1-§19.3.

### 5.1b Reference counting
Never Release a pointer you did not AddRef or create. Use `FC4_RELEASE_CREATED` for temporaries the game creates for us, `FC4_ACQUIRED` / `FC4_RELEASE`
for references we take. In a simulator-Shutdown refaudit, NEGATIVE per-class lines are a window artifact (the budget
summary resets the counters); trust the per-object `over-release` / `released last reference` warnings.
`Read` runs before `Init`: never size saved state from the city inside `Read`.

### 5.2 x87
The game runs its main thread at 24-bit precision; our DLL's presence raises it to 53-bit during app init, and the
plugin re-asserts 24-bit at PostAppInit (foundations §18). Declaring `float`/`double` where the game returns nothing
leaves values on the x87 stack and eventually produces NaN sizes; `docs/abi/README.md` states the return rule.

### 5.3 Differentials
Separate runs of the original are not deterministic, so compare with **same-process shadows**: our core runs on the
original's live inputs at the same moment, read-only, and the results are diffed bitwise (budget, demand and every
grid sim have one). Separate-run soaks are only for long-run paths.

### 5.3b Demand
Operation order matters for bitwise agreement (demand.md §8.1). For unspecified behaviour, follow the engine
prompt's `SPEC-GAP` rule. The coordinator can assign RE follow-up when a difference needs a spec change.

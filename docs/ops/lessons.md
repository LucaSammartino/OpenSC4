# 5. Lessons that engine comments and lane prompts cite

Moved from HANDOFF §5 on 2026-10-20 with the same numbering: "HANDOFF §5.1" in an engine comment or a lane
prompt means §5.1 here. Who reads it: engine lanes (and anyone touching the plugin or reading a shadow).

### 5.1 Threading
The game calls our getters from a second thread while the main thread is inside `OnTick`. Shared state that getters
read must not live in a `RefCell` (run 23's crash). Details: foundations.md §19.1-§19.3.

### 5.1b Reference counting
Never Release a pointer you did not AddRef or create (run 37: a borrowed disaster manager released on ignition freed
the fire object). Use `FC4_RELEASE_CREATED` for temporaries the game creates for us, `FC4_ACQUIRED` / `FC4_RELEASE`
for references we take. In a simulator-Shutdown refaudit, NEGATIVE per-class lines are a window artifact (the budget
summary resets the counters); trust the per-object `over-release` / `released last reference` warnings.
`Read` runs before `Init`: never size saved state from the city inside `Read`.

### 5.2 x87
The game runs its main thread at 24-bit precision; our DLL's presence raises it to 53-bit during app init, and the
plugin re-asserts 24-bit at PostAppInit (foundations §18). Declaring `float`/`double` where the game returns nothing
leaves values on the x87 stack and eventually produces NaN sizes (run 74); `docs/abi/README.md` states the return
rule. Ghidra renders the 42-byte FPU helper `0x008929E0` as an empty function, and `PreFrameWorkInit` does not
decompile: read the bytes on that path.

### 5.3 Differentials
Separate runs of the original are not deterministic, so compare with **same-process shadows**: our core runs on the
original's live inputs at the same moment, read-only, and the results are diffed bitwise (budget, demand and every
grid sim have one). Separate-run soaks (`tools/compare_soaks.ps1`) are only for long-run paths.

### 5.3b Demand
The demand shadow closed a 4-ulp difference by matching the exe's operation order (demand.md §8.1). The method that
worked: the engine lane turns a difference into a precise question, an RE lane answers it into the spec, an engine
lane implements it.

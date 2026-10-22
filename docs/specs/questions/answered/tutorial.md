# tutorial: answered questions (archive)

<!-- from: glue-tutorial4 (2026-10-21) -->
- **TU7 - section 4.3 (the debug cheat's subcommands):** the cheat `0x6A382BAF` "Tutorial" handler (`0x0043F870`,
  message `0x230E27AC`) has subcommands reset, save, step, next, quit. Where in the message is the subcommand
  (the cheat string, data field 1/2, a `cIGZString`?), how is it parsed, and what does each one do (which slots /
  fields / registry and window calls, in order)? Only reachable when `cISC4App` slot 21 is true. [the handler is
  registered as specified, logs the cheat and ignores it]
  **ANSWERED 2026-10-22 (re-tutorial-contracts4):** DIFFERS — §4.3.1 proves the plain notification layout
  (+0 kind,+4 cheat ID,+8 borrowed string interface), native tokenization, lowercase whole-token comparison,
  exactly-three-token reset/save/step versus exactly-two-token save/next/quit/reset, numeric conversion and
  ordered field/interface/window effects. Registration alone checks app21; the handler itself does not.
  Arity, save validation and reset ordering are observable and ignoring the cheat omits those effects.

<!-- from: glue-tutorial4 (2026-10-21) -->
- **TU9 - sections 4.2 / 4.4 (the tutorial window as direct calls):** `cSC4TutorialUIWinProc` (100 bytes,
  constructor `0x00442920`, close `0x00442C80`, `OnTutorialEvent` `0x00443970`) has no interface or vtable rows in
  docs/abi. To keep the original window a replacement needs: the allocator for the 100 bytes (docs/abi/allocators.md),
  the constructor's arguments and the reference count it leaves; which pointer `startTutorial` stores at +0x40 and
  hands to `Init` (and `Init`'s arguments, e.g. the tutorial system, the GUID, the task number, the Lua context); the
  window's vtable with slots 6-11 (`FilterMessage` field, `ShowUIHintsForUsers`, `HandleCatalogMenu`, current task,
  `FilterLeftMouseClick`, `ResetTutorialNumAndIndex`): address, pop, arguments and return channel; and the close
  call's convention. Does the window call back into the tutorial system (which slots) during Init or a task step?
  [no window is created: every slot answers its without-window column]
  **ANSWERED 2026-10-22 (re-tutorial-contracts4):** DIFFERS — §4.2.2 and §4.4.1 give the native
  boundary: heap C(100), constructor refcount0/base result/no arguments, two explicit owner references,
  Init/close thiscall/no arguments/pop0, event uint32/pop4/AL true, base table00A8638C slots6-11. Init obtains
  system slots7/12/11 from borrowed global00B20E88; task advancement uses slots12/11/20/11, and completion posts
  the ordinary end message. There is no Init owner/GUID/Lua/flags argument. Slot10 takes coordinate addresses.
  ABI proposal: build-runs/abi-rows-re-tutorial-contracts4.tsv. Scope is the tutorial owner's external window
  protocol; no complete UI-class implementation/coverage or runtime-reopen verification is claimed.

<!-- from: glue-tutorial4 (2026-10-21) -->
- **TU8 - section 4.4 slot 13 (`FilterMessage`'s argument):** "a message of kind 3 with ID `0x287259F6` /
  `0x287259F7` ... data field 2 ... `0x49D993D4`, its field 3": what is the argument's type (a `cGZMessage*`, a
  `cIGZMessage2*`, a window message struct), where are "kind", "ID" and the data fields (offsets, or which
  interface slots), and what are their types? [not decoded; the answer is true, which is the without-window answer]
  **ANSWERED 2026-10-22 (re-tutorial-contracts4):** DIFFERS — §4.4.2 proves a plain
  window-message prefix of four dwords, kind+0, ID+4, payload+8/+12, no vtable. Kind3 with F6/F7 IDs forwards
  +8, with ID49D993D4 forwards +12 to original-window slot6. All returns true, but forwarding can advance a task
  and call back into system slot20, so an unconditional no-op differs when the window is present.

<!-- from: glue-tutorial4 (2026-10-21) -->
- **TU6 - sections 4.2 / 4.3 and TU1 (registry calls as direct calls):** `cSC4TutorialRegistry` has no vtable (TU1),
  so a replacement `cSC4TutorialSystem` can make `startTutorial`'s `SetActive(GUID, 1)`, `endTutorial`'s
  `SetActive(GUID, 0)` and the city-init handler's entry lookup only by calling `0x004B5D80` and `0x004B5D10`
  itself, in the docs/abi/direct-calls.md format, but neither is there. For each: calling convention (thiscall with
  ECX = `cISC4App` slot 49's result, `&app+0x1C0`?), the stack arguments (count, types: is the GUID a `long` passed
  by value, is `active` a bool byte or a dword?), the bytes the callee pops and the return channel (AL / EAX; for
  `0x004B5D10` a `cItem*` into the list node or null?). Is calling `SetActive` from a DLL safe while the
  preferences block may be saved (slot 17) on the main thread only? [no call is made: SetActive is not made and
  the entry lookup answers "no entry", so a loaded tutorial city keeps is-tutorial set without a window, quirk 2's
  state]
  **ANSWERED 2026-10-22 (re-tutorial-contracts4):** DIFFERS — tutorial.md §4.2.1 proves ECX = app slot49's
  borrowed result (already app+0x1C0), GUID by value, GetItem pop4/EAX borrowed item-or-null, SetActive pop8/AL
  changed-state with active read as a byte in a dword slot. Preferences save is synchronous only when an array
  cell changes; no concurrency guarantee or lock is supplied. Calling within the original tutorial/city execution
  flow can perform the lookup/state change; answering no-entry/skipping the call does not match that flow.
  Original question also requested direct-calls.md entries; proposed rows are in
  build-runs/abi-rows-re-tutorial-contracts4.tsv for coordinator gating (no ABI tables edited).
<!-- Archive for [tutorial.md](../../tutorial.md). Lanes do not read this file. Open questions are in ../tutorial.md. -->

<!-- from: From the prop manager, lot configuration manager and tutorial cores (2026-10-16) -->
- ~~**TU4:** (tutorial.md §1) what does the shared advisor-system Release do on the last reference? [marks the object
  destroyed without running Shutdown or closing the window *[inferred]*]~~ **ANSWERED 2026-10-16 (re-answers25):** **DIFFERS**: it destroys the object, it does not just mark it. Release (`0x00449310` → `0x0090D997`) works on the reference-count sub-object at +0x14 (count at **+0x18**, not +0x14); at the last reference it calls the scalar deleting destructor (`0x0043F6C0` → `0x0043F780`, flag 1), which runs `0x0043F6E0` and frees the 0x44-byte object. The destructor releases the window (+0x40) **once** without closing it (`0x0043F70F`–`0x0043F718`), frees the vector buffer at +0x34, releases the Lua context (+0x30), runs the sub-object's base destructor (`0x0090D990`) and clears the global instance pointer `0x00B20E88` (`0x0043F746`). "Without running Shutdown or closing the window" MATCHES: no unsubscription, no cheat unregistration. A window that is still open keeps one of its two references (leak, *[quirk]*). tutorial.md §1.

<!-- from: From the prop manager, lot configuration manager and tutorial cores (2026-10-16) -->
- ~~**TU5:** (tutorial.md §4.3, "Shutdown undoes both") what exactly does Shutdown undo? [unsubscribes the three
  messages and, when a cheat manager exists, unregisters the cheat and its handler *[inferred]*]~~ **ANSWERED 2026-10-16 (re-answers25):** **DIFFERS**: the bracket misses the window. Shutdown (`0x0043F330`) does, in order: (1) when the cheat manager `0x00B43CAC` exists (no `cISC4App` slot 21 test, unlike Init): slot 6 with (handler +8, 0), then slot 4 with `0x6A382BAF`; (2) when a window exists (+0x40): close (`0x00442C80`), release, clear +0x40, release again, the same window teardown as endTutorial; (3) when the message server `0x00B43CCC` exists: unsubscribe `0xAA3840E6`, `0xEA8AE29A`, `0x26C63343`; returns true. Is-tutorial, end-pending, the registry and the 3D view are not touched. tutorial.md §4.3, test 7.

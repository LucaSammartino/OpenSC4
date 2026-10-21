| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-span1555-impl4: cSpanRender1555 native bridge (enable=span1555) | `glue-span1555-impl4` / engine lane | ready: vtable 0x00AC182C slots 1-5 forwarded, original vs ours per call; constructed COM cGZBuffer fixture 158/158 exact (gs1555i4-r2 clean, ref 0); gate cargo 3947/0, clippy 0, ctest 64/64; SR1555-7 filed | 1. RE answers SR1555-7, then drive DrawPreclippedSpanList (native caller) on a constructed buffer. 2. SR1555-6: game_calls stayed 0 in a 120 s session; a longer/UI session could confirm no shipped producer. |

## Report

What changed
- engine/capi/src/span1555.rs (+ tests, fc4.h decls, lib.rs additive): `fc4_span1555_{solid,alpha,gradient,alpha_gradient,blt}`,
  the five methods over explicit memory windows (codes 0..7, -1). Tests: both section 1.1 conformance examples + error codes.
  ABI exports added; FC4_ABI_VERSION not bumped.
- engine/plugin/compat/FC4Compat.{h,cpp}: `Span1555SlotEntry(slot)` = vtable 0x00AC182C entries 0-5 and the functions they
  must hold (0x0082FF00, 0x008310B0, 0x00831100, 0x00831200, 0x008312D0, 0x00831470), identified build only.
- engine/plugin/FC4Span1555Shadow.{h,cpp}: with `enable = span1555` (subsystem key, 0 classes, default off) entries 1-5
  are patched (only if each still holds its published function) with __fastcall/EDX-dummy forwarders = thiscall,
  callee pops 16/16/20/16/24. Per call: snapshot the span-covered destination bytes, run the original, keep its bytes,
  restore, run ours, compare byte for byte; ours is kept (replacement), the original's bytes are put back if ours
  refuses the input; n = 0 or off-row spans go to the original alone. RenderBlt's source window = the readable
  VirtualQuery region around the record pointer (or the first texel).
- Fixture at first city load, labelled "constructed buffer, not a game path": cGZBuffer via GZCOM GetClassObject
  CLSID 0xC470D325 (FC4_ACQUIRED / FC4_RELEASE_CREATED), destinations type 4 and 6 (32x8, 16 bpp, Lock 0x8040),
  sources type 7, 6, 9 (16x16, Lock 0x800); calls go through the patched vtable entries with a 4-byte object holding
  the vtable pointer: 2 solid, 3 alpha, 1 gradient, 1 alpha gradient, 72 blits (3 formats x key off/on x 12 kinds:
  plain, stretch du=dv=0x8000, tiled 8x8 at (2,3) u0=3 v0=5) per destination type.
- FC4Subsystems.cpp / FC4Observer.cpp / CMakeLists.txt: additive wiring.

Measured
- Game gs1555i4-r2 (Konradshohe, 120 s normal speed): verdict clean, ref_warnings=0. Fixture total cases=158
  exact=158 diff=0 ours_refused=0. Per slot (calls/exact): Solid 4/4, Gradient 2/2, Alpha 6/6, AlphaGradient 2/2,
  Blt 144/144; original-changed bytes 911/794/1200/784/32669 (non-trivial drawing). Buffer FNV type 4
  0xACF6FDC5 -> 0x28930FFE, type 6 0x7DD59B25 -> 0xC5D4E1EA. game_calls=0 for all five slots (no non-fixture caller
  reached the class in that session; SR1555-6 evidence, not proof).
- gs1555i4-r1: same 158/158 exact but ref_warnings=16 (buffers released without FC4_ACQUIRED); fixed in 929d49da.
- Gate (qbuild all): cargo test 3947 passed / 0 failed, clippy 0, CMake build PASS, ctest 64/64.

Not done / questions
- Native caller path (DrawPreclippedSpanList via draw system slot 4 + context Init) not driven: the constructed
  context's initial state, setter slots and lock/unlock contract are unpublished -> SR1555-7. Unlock uses the lock
  flags (literal reading, commented). Shipped producers remain unproved (SR1555-6).

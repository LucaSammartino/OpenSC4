# The draw system service (`cGZDrawSystem`) (behaviour spec)

Scope: the owner `cGZDrawSystem` (17 functions, 1,259 bytes) as tools/coverage.py assigns it. Its context creation
paths are already in span-render-4444.md §1 and span-render-8.md §10.5 (`Init` `0x0082BC90`, slots 3–5); this spec
adds the service, the context registry and the idle flush.

Related: span-render-4444.md §1.1–§1.3 (slot 4 `CreateDrawContext`, the factory list and the RenderToBuffer factory),
span-render-8.md §10.4–§10.7 (slot 5 and the software contexts), widget-toolkit.md (`create_private_buffer`),
framework-gz-services.md (system services), framework-misc2.md §5.1 (the lock).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the interface table `0x00AC1668`, `OnTick`,
`Shutdown` and the flush bodies were read in the disassembly. Written 2026-10-24 (re-drawsys7).

## 1. Object *[compat]*

52 bytes (`GZCOM_CreatecGZDrawSystem` `0x0082BDE0`; constructor `0x0082BBF0`): the draw-system interface at `+0`
(vtable `0x00AC1668`, IID `0x4B2EA3B3`; `QueryInterface` `0x0082B890`, else the system service's), a system service
at `+4` (service ID `0xEB2EA3CB`, priority 1,500,000), a third face at `+8`; the factory list `+0x20`, the context
list `+0x24` (both empty circular lists), a tick counter `+0x28` (0) and a critical section `+0x2C`. Thunks
`0x0082BC60`, `0x0082BC70` (QueryInterface), `0x0082BC80` (deleting destructor); destructor `0x0082BE00` (the lock,
then both lists).

## 2. Interface *[compat]*

| Slot | Address | Behaviour |
|---|---|---|
| 3 `AddFactory(f)` | `0x0082BD30` | at the front of the factory list (span-render-4444.md §1.2) |
| 4 `CreateDrawContext(obj, &out)` | `0x0082BAB0` | span-render-4444.md §1.3 |
| 5 | `0x0082B980` | span-render-8.md §10.5, span-render-4444.md §1.2 |
| 6 `RegisterContext(c)` | `0x0082BD90` | under the lock, `c` is inserted at the front of the context list (no reference taken) |
| 7 `UnregisterContext(c)` | `0x0082BAF0` | under the lock, the first entry equal to `c` is removed |
| 8 `FlushIdleContexts(n)` | `0x0082BB40` | under the lock, every registered context whose slot 9 is true and whose slot 11 (the tick it was last used) is at least 5 ticks behind the counter gets its slot 6; the argument is not used (the threshold is the constant 5) |
| 9 `FlushAllContexts()` | `0x0082BBB0` | under the lock, slot 6 of every registered context |

## 3. Service *[compat]*

- `OnTick` (`0x0082B8E0`): when the tick counter is a multiple of 5, `FlushIdleContexts(5)`; then the counter is
  incremented; true. So idle contexts are flushed every fifth tick.
- `Shutdown` (`0x0082B8C0`): removes the service from the framework's tick list (framework `RemoveFromTick`, slot 10);
  true. The lists are kept until destruction.
- `Init` (`0x0082BC90`) is span-render-8.md's (the RenderToBuffer factory appended at the tail).

## 4. Function inventory

`cGZDrawSystem`: `0x0082B890`, `0x0082B8C0`, `0x0082B8E0`, `0x0082B980`, `0x0082BAB0`, `0x0082BAF0`, `0x0082BB40`,
`0x0082BBB0`, `0x0082BBF0`, `0x0082BC60`, `0x0082BC70`, `0x0082BC80`, `0x0082BC90`, `0x0082BD30`, `0x0082BD90`,
`0x0082BDE0`, `0x0082BE00`.

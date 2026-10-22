# animation-coordinator: open questions
<!-- Questions about [animation-coordinator.md](../animation-coordinator.md). Append new entries at the END; when one is answered, move it to answered/animation-coordinator.md. Rules: README.md. -->

<!-- from: re-animcoord (2026-10-19) -->
- **AC1 - §1.1, §3:** what message `0x231BBF91` is, and what the byte `+0xF16` of the global `0x00B43CD8` holds (an
  animation detail setting is inferred; animation needs it at 2 or more).

<!-- from: re-animcoord (2026-10-19) -->
- **AC2 - §2:** the exact test applied to an occupant whose exemplar has property `0x8A5F88E3` (a funding check
  against its budget item is inferred).

<!-- from: core-animation-coordinator4 (2026-10-22) -->
- **AC3 - owner identity and method catalogue, §0–§1 and §6:** the implementation task requests
  `cS3DAnimationCoordinator` (45 functions, 7726 bytes), but the supplied brief names
  `cSC4AnimationCoordinator` and the permitted public sections describe occupant visibility/power/budget gating.
  Which public spec owns `cS3DAnimationCoordinator`? Please publish its exact class name, all 45 method names,
  signatures and function-to-contract checklist, and clarify whether it is a different owner from the occupant
  coordinator. [No substitution of the occupant coordinator for the requested owner; all 45 requested functions
  remain unimplemented pending the public contract.]

<!-- from: core-animation-coordinator4 (2026-10-22) -->
- **AC4 - requested timeline owner behavior, §0–§1 and §6:** no public contract in the supplied sections defines
  `cS3DAnimationCoordinator` registration/reference ownership or lifetime, scheduler cadence, play/pause/stop,
  looping, transitions, frame selection, callback order, reentry or failures. Please specify initial state and
  return values; raw tick/counter types and overflow/boundary rules; tick-to-frame and transition decisions;
  duplicate registration/removal and AddRef/Release behavior; callback mutation and failure ordering, with
  independent tick/loop/transition/reentry/failure/reference conformance cases. Which external time, resource
  and animation-object primitives are foreign provider operations? [No default flags, time conversion,
  scheduler policy, reference policy or callback ordering is invented; these requested behaviors remain blocked.]

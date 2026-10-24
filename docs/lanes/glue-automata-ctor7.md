| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-automata-ctor7 (owners capture references and construction codec) | Codex / glue-automata-ctor7 | in progress | Audit references and construction record; gate and three house shadow runs to 2032. |

## Handoff
- Goal: fix owners capture reference warning and construction occupant 254/250 byte mismatch.
- Worktree reused at reserved detached HEAD; no branches or main changes permitted.
- Prior crash6 report read; its narrow automata-only runs passed but house shadows exposed regression.
- Implemented: capture pin ownership per entry; separately counted replay acquisitions,
  guarded releases, and abandoned replay cleanup. Added malformed/truncated/failed writer tests.
- Filed LR13: native construction writer identity/QI/temporary lifetime contract missing.
  Kept literal IID checks; no guessed non-null coercion. Added unavailable-IID diagnostics.
- Next: collect running qbuild all (exec session 5260), then queued verify and diagnostic house run.
- Required gates: stable final qbuild all and queued verify; three unique turbo house shadow runs to 2032.
- No requests outstanding; no questions filed yet.

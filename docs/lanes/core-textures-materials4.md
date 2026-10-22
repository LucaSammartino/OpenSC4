| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-textures-materials4: portable textures and materials | Codex / `core-textures-materials4` (engine lane) | in progress; reserved detached worktree reused; main baseline already gated | 1. Audit published owners and existing primitives. 2. Implement and test. 3. Full gate and report. |

## Handoff
- Goal: implement published portable behavior of cS3DTextureBindingFactory, cS3DTextureBinding, cS3DMaterialManager, cS3DMaterial, core only.
- Done: clean worktree verified; spec sections 1-3 and 7 read; no pre-edit full baseline per task override.
- Next: audit existing core/formats and ABI owner inventory, then implement focused owners and independent tests.
- Questions currently TB1/TB2 open; index next free TB4. No game requests (portable core only).
- No CAPI/plugin/ABI/map edits; no complete-owner credit until body inventory is reconciled.

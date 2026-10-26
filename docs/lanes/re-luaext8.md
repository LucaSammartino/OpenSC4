| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-luaext8: the `fn:LUAExtension_*` owners and `SCLuaTableRef`, lua-game-extensions.md (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`, after re-scrcmd8) | **ready** (docs only). Eighteen owners claimed: specified +3,406 bytes (+21 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
The bodies advisors.md §8.3 only summarises were read in the disassembly with their string literals. Written on
2026-10-25.

### What changed

- docs/specs/lua-game-extensions.md (new): §1 each owner and the section that specifies it (the camera, tool and
  window bodies are already advisors.md §8.3.1–§8.3.2); §2 the common shape and `_combine_guids`; §3 `advice_info`;
  §4 `ordinance_name` / `ordinance_description` (the placeholder text is overwritten by the getter's string); §5
  `trigger_event`; §6 `unlock_reward_building` and its status-record helpers; §7 `SCLuaTableRef`; §8 conformance
  tests.

### Coverage

- tools/coverage-map.tsv: the `fn:LUAExtension_*` row now names advisors.md and lua-game-extensions.md; the
  `SCLuaTableRef` row names lua-game-extensions.md.
- Specified bytes 5,127,500 → 5,130,906 (+3,406, 91.9%, `cov_local.py` with owners-c1).

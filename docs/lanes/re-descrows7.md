| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-descrows7: coverage rows for three free functions that existing specs fully describe (RE coverage lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-tokens7) | **ready** (map only). `GenericBufferClip`, `GenericBufferClipStretch` (gz-graphics.md §6.3–§6.3.1) and `SC4LoadString` (crime.md §7, "Name"); specified +1,378 bytes (+3 functions) | none |

## Report

Each function is the subject of a full behavioural description in an existing spec, cited by address:
`GenericBufferClip` (`0x00930BE0`, gz-graphics.md §6.3), `GenericBufferClipStretch` (`0x00930CF0`, §6.3.1) and
`SC4LoadString` (`0x00603040`, crime.md §7 steps 2–4 and the failure rule). Neither spec withholds owner credit. No
spec text changed. Written on 2026-10-24.

### Coverage

- tools/coverage-map.tsv: `fn:GenericBufferClip`, `fn:GenericBufferClipStretch` → gz-graphics.md;
  `fn:SC4LoadString` → crime.md.
- Specified bytes 4,887,561 → 4,888,939 (+1,378, 87.6%, `cov_local.py`).

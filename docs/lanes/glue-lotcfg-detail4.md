| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lotcfg-detail4: lotcfg DetailLevel host reads the city-detail preference (prop-lotconfig §B3.1) | `glue-lotcfg-detail4` / (engine lane) | in progress | see Handoff |

## Handoff

- Goal: cFC4LotConfigManager DetailLevel host returns actual SC4Preferences videoPreferences.cityDetail (+F16) instead of hardcoded 3; instrument; game run with current38+citydetail to 2032 at non-high detail.
- Done: lane file.
- Next: read prop-lotconfig B3.1/B3.2, city-detail.md §2, cFC4LotConfigManager.cpp DetailLevel, cFC4CityDetail.cpp GetPreferences.
- Facts: (none yet)

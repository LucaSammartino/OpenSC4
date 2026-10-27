# cSC4WinMiniMap::cTZMap

| Spec section | Code item | Worked case / test |
|---|---|---|
| minimap 3, `cTZMap::Init` | `TzMap::init`, `free` | `same_dimensions_keep_pixels_and_free_allows_reallocation`; MM-C2/C3 in parent tests |

Dimensions stay stored when pixels are freed. Allocation of the same live size
preserves its contents; replacing a size allocates a new block. Newly allocated
contents are a documented SPEC-GAP (zero until terrain rendering).

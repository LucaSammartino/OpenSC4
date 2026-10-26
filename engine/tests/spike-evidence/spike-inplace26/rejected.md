# Selection and rejections

This is a deliberately single-family 20-function cohort. It does not establish
the rate for all closed functions, UI/audio, or any other owner. No function is
credited merely because its Rust unit tests pass. Missing ABI/coverage is not a
numeric mismatch. Each selected entry and its body size is in FC4Inplace.cpp,
from view-utilities.md sections 12-13. Runtime decoding may additionally reject
selected entries, with `installed=0`; these remain in the denominator of 20.

Other brief candidates considered but not adapted:

| Function | Why not included |
|---|---|
| cRZFastCompression3::CompressData | Variable buffer/length and compression transport not adapted in this cohort; vtables.tsv does publish four argument slots. |
| cRZFastCompression3::DecompressData | Same transport work; data-formats 2.7 specifies malformed native out-of-bounds behaviour needing quarantine. No claim that its ABI is absent. |
| cSC4LightingManager::CalcShadowTerrainRect | Lighting 5.7 reads a terrain-height grid behind the receiver; requires transitive grid capture beyond the fixed receiver. |
| cSC4LightingManager::GetVertexColors | Lighting 4.4 has counted strided arrays and a matrix; additional native layout adapter not built. |
| cS3DModelInstanceAnimatedSprite::UpdateView | Different large native owner/context geometry adapter; not selected. |
| cS3DModelInstanceAnimatedSprite::GetTextureData | Frame/resource pointer graph capture not built. |
| cRZLanguageManager::GetLanguageFromSystemLanguage | Integer-return alternative with published slot/argument count; not selected, not ABI-blocked. |
| Fullpath | paths 1.2 includes null-output malloc and stack-dependent before-buffer underflow; requires bounded string/domain handling. Not selected. |
| cHTMLDocument::BuildFirstSets | html-document 4.1 explicitly writes global std::set trees. Not self-contained in receiver/argument windows; excluded rather than called. |

Other view-utilities section 12 entries, all explicitly not credited:

| Entry | Reason |
|---|---|
| point-array AABB | Variable counted window; existing local transport only holds three points. |
| MinOfFourFloats; MaxOfFourFloats | x87 return preservation not implemented; float32 observer return could perturb game results. |
| ViewportUnitsToWorldUnits; ProjectPointToViewportZ | Same x87 return preservation limitation. |
| IsBoundingBoxInDrawFrustum | Existing cached Rust adapter imports only the view frustum; draw-state import absent. |
| TrianglePick | Additional mutable Pick/vertex adapter not selected. |
| RayIntersectsGroundPlane | Additional stdcall/boolean adapter not selected. |
| GetObjectYRotationAngle | x87 return preservation limitation. |
| GetVectorForEdge | Lower-priority alternate local helper; not selected. |
| RoundDownToNearestBlockBoundary; RoundUpToNearestBlockBoundary | x87 return preservation limitation. |
| SC4ViewUtilities constructor | Writes owner, outside the existing read-only cached adapter. |
| GetCameraViewAnglesForRotation | Additional stdcall multi-output adapter not selected. |
| GetFrustumAABBCityClipped | Cached frustum clipping adapter not present in the reused private Rust transport. |
| GetCityWidthFromOrthoView | x87 return preservation limitation. |
| UpdateCameraPosition | Provider calls and owner writes (storage W). |
| ViewportPositionToRay; GetViewportPointIntersectionWithGroundPlane | Borrowed camera provider calls (P). |
| MeshPick; ModelPick | Resource graph/transform traversal, not fixed captured windows. |
| GetGroundPlaneVisibleAreaRectangle | Owner writes / cached-area construction adapter (W). |
| GetViewportRectGroundCorners | Camera-provider dependency (P). |
| BuildDrawFrustumDataFromViewportRect | Owner writes / provider construction path (W). |
| primitive triangle count | Integer-return alternate helper, not selected. |
| iterator deleting destructor | Deallocation, not reversible replay. |
| iterator vertex copy; index decode; iterator select group; iterator next | Private iterator/mesh graph (I), not adapted. |

The x87 exclusion reasons above describe the initial selection. The corrected
bridge preserves extended native x87 results; a comparison transport for those
scalar values is still absent and the cohort is kept unchanged. Rotation and
visible-edges remain rejected by the fail-closed entry decoder on both clean
round-2 runs; contracts exist, but no compatible patch was installed.

No new Rust algorithm was needed for the selected 20; new implementation time
for missing Rust algorithms is zero. Existing cached-owner and word-transport
adapters are reused without treating their expected outputs as native truth.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-glyph-textures4: glyph and string texture portable owners | Codex / `core-glyph-textures4` (engine lane) | in progress | Audit published inventory, implement owners, focused checks and final full gate. |

## Handoff
- Goal: published portable core for cSC4GlyphTextureManager, cSC4GlyphTextureInfo, cSC4StringTextureInfo; core only.
- Worktree reserved and detached; commit normally, never change refs. Baseline is already gated by coordinator; no pre-edit full gate.
- Done: scope commit; audited full published sections 1–3/7, ABI rows, existing core/formats and font/string SDK primitives.
- Implemented: glyph map/sharing/UVs/refcounts, code-page array append, lazy strings/cache/order/sweep, restore and shutdown, 19 focused tests and independent size vectors. Reuses fonts::FontRef through CoreFont.
- Questions appended: GT3 numeric boundaries, GT4 missing body inventory/lifecycle, GT5 restore outputs/failure, GT6 output/reference/invalid inputs. Manager completeness withheld; inherited/shared slots are absent from spec.
- Next: quick gate running (exec session 67801); wait for result, fix failures, formatting check; final qbuild all once stable.
- Core source: engine/core/src/glyph_textures/{mod.rs,tests.rs,size-vectors.txt}; no CAPI/plugin/ABI/maps touched.
- No game requests; core-only task has no bindings/shadow key.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-glyph-textures4-r1-manifest.json; budget 150000, maxCalls 100.

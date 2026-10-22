# Public owner method checklist

Sources: overlay-views.md §§1–3; network-overlay-views.md §§1–2. Core only, unbound.
The six exact owners are below. The brief's 54-function inventory is not a public
method list; OVV6 requests the missing catalogue. No complete-owner credit yet.
Terrain-view §8 describes terrain-manager methods, intentionally outside this module.
No terrain replacement or TR33–TR37 contract is changed.

| Owner | Published method | Implementation / remaining contract |
|---|---|---|
| cSC4PathView | Init | Semantic plain/textured states; lifecycle OVV6 |
| cSC4PathView | SetThickness | Explicit positive finite t; invalid inputs OVV2 |
| cSC4PathView | SetPathTexture | Acquired binding, captured per path; failure semantics OVV6 |
| cSC4PathView | NewPath | Stable ascending priority, literal ID/flags, newest independent of order; attributes OVV2 |
| cSC4PathView | AddPathVertex | x-z squared-distance suppression; equality OVV2 |
| cSC4PathView | ClearPaths | Drops owned paths/texture references |
| cSC4PathView | endpoint / joint helpers | Unit normal/bisector/fallback; side/miter OVV3 |
| cSC4PathView | DrawPath | Explicit-input ribbon; attributes/eligible arrows can return gaps |
| cSC4PathView | DrawArrowhead | Blocked OVV3 |
| cSC4PathView | Update | Ordered draws, identity, offset 15/restoration, texture wrapping/plain restoration |
| cSC4PathView::PathInfo | creation / storage / lifetime | Literal fields and owned texture; defaults/public surface OVV2/OVV6 |
| SC4MarkedCellView | SetCellSize | Size/inverse; invalid inputs OVV5 |
| SC4MarkedCellView | SetShouldIgnoreDepthBuffer | Flag and semantic state |
| SC4MarkedCellView | SetShouldUseWaterAltitude | Flag and 16m corner heights |
| SC4MarkedCellView | SetCellColor | Blocked OVV4; byte helper does not claim packing |
| SC4MarkedCellView | Init | Semantic state, acquired foreign lot filter; filter construction OVV5 |
| SC4MarkedCellView | DrawCells | Bounds, frustum, lot/water, split, colours, ≤1024 vertices/batch; edges OVV5 |
| SC4MarkedCellView | Update | Identity, offset 4/restoration, normal then hidden additive/greater pass |
| cSTEVertBufExtn | GetVertexBufferExtension | Bool 25 gate, IID/format, validation/release, row capacity; OVV7 |
| cSTEVertBufExtn | CreateBuffer | Flush active lock, lock/reset; explicit failure; lifecycle OVV7 |
| cSTEVertBufExtn | row-copy helper | Two compact rows of n+1 raw 32-byte vertices |
| cSTEVertBufExtn | SetIndexBuffer | Owned foreign terrain source; width/layout OVV7 |
| cSTEVertBufExtn | DrawPrimsIndexedUsingVertBuf | Bounds, compact rebase, append, capacity flush/relock |
| cSTEVertBufExtn | FlushBuffer | Unlock then triangles rounded down /3; no duplicate unlock on failure |
| cSTEVertBufExtn | ReleaseVertexBufferExtension | Owned buffer release, no implicit draw; locked-release OVV7 |
| SC4PathInfoView | Init | Semantic state; fields/format NOV2 |
| SC4PathInfoView | Update | Pulse, cell/occupant traversal, byte filter, geometry; colours NOV2 |
| SC4PathInfoView | path draw | Cap/height, known colours, cross, arrow/bars; details NOV2 |
| SC4TrainJunctionView | Init | Blocked NOV3; resource helper does not replace Init |
| SC4TrainJunctionView | GetPathBitmap | Explicit entry direction, chord classifier, literal keys; details NOV3 |
| SC4TrainJunctionView | GetArrowBitmapQuad | Blocked NOV3 |
| SC4TrainJunctionView | DrawTrainJunction | Blocked NOV3; transparency helper implemented |
| SC4TrainJunctionView | Update | Own traversal; fails NOV3 even for empty frames |

Rust constructors/snapshots/explicit-data helpers are core APIs, not invented native
slots. No guessed packed state/default colour/key names/line format/transform/model
choice or arrow geometry is published. Provider outputs are foreign primitives only.

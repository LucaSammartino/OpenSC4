<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Lot model settled public contracts

Authority: `docs/specs/lot-models.md` sections 2.1–2.3, 3.1–3.2,
4.1–4.3 and 8–8.2. This extends the existing owners with bounded LM3–7
bodies. No native execution, complete-owner coverage, ABI integration or
final GPU equivalence is claimed.

| Owner | Implemented bounded body | Foreign boundary |
|---|---|---|
| Foundation | Ordered five position/BGRA/UV quads, footprint-width side U, stored reciprocal top UVs; border enumeration, ground-before-water, truncating existing/required counts; geometry rebuild without relight; zero/mutable fallback keys and independent top draw | Individual terrain/normal-light outputs and acquired resource leases |
| Wall | Four biased cell ranges, inner/across mapping, raw query order and flat suppression; three emit branches, world UVs, seam positions/raw bounds; retained tails/full-buffer BGRA recolour; resource/sort/facing/draw decisions | Individual lot/terrain/light outputs and acquired leases |
| Cell | Native strip and orientation/postmirror tables; unsigned-byte placement, slot order/bounds/BGRA; first/later zoom and rotation updates, UV-only SetTexture; wrapped-key lookup; packed states, modes 0–3 combiner bytes/decoder and ordered draw requests | Context applies packed state/driver calls; terrain view draws nonflat cells |

## Domain and exclusions

Stored values are binary32; explicit binary64 register operations select
round-to-nearest/ties-to-even PC53. There is no ambient-control-word or
general PC64 equivalence claim. The wall lattice helper uses bias/store/bits,
not floor/ceil, for nonnegative coordinates through 2^24. Wall ledgers are
published PC53/64 analytical examples, not native observations. Cell world
coordinates are unsigned bytes. InvalidInput denotes unsupported nonfinite,
overflowing, invalid-mode, unbacked-element, foreign-error or allocation
domains, not guessed native false/clamp/rollback behavior. Foundation
scale<=0 is excluded because unchecked division leaves the finite UV domain;
it is not a native guard.

LM1 upstream authors/other modes, LM2 object/base/QI/pool/lifetime and LM8
residual arithmetic remain open. Explicit inputs do not invent constructor
defaults. Each successful factory call supplies an acquired Rust lease;
adapters normalize raw foreign replies. Borrowing releases nothing.
Replacement/drop releases acquired leases once. Rust Drop is a safety policy,
not native reference-count, pointer-identity or cleanup proof. No ABI changed.

## Entry points and compatibility

Foundation `adjust_from_terrain` owns enumeration and `recolour` writes BGRA
from integer light. Legacy `setup_vertices`/`draw` retain semantic floating
colour metadata; that compatibility metadata is not native packed-colour proof.

Wall `rebuild`/`setup_from_bounds` own traversal. `triangles()` returns the
full stored list including tails; `draw_lit` recolours it. Legacy `draw`
reports pending recolour when no raw light was supplied. Vec capacity is a
portable allocation choice, not a native heap guarantee.

Cell `from_record`, `update_vertices` and `draw_owned` are the settled path.
`CellCommand::State` selects the owner tuple; the foreign context owns
changed-bit device calls, default wrap/filter/transform-clear requests and
render overrides in section 4.3. Command errors retain the issued prefix.
Legacy `new(record,layout)` ignores the historical layout oracle; the
standalone layout helper remains for compatibility/conformance vectors.
Legacy semantic `place`/`draw` are not the ordered callback path.

## Independent fixtures

The six original lm01–lm06 conformance files remain; lm01 now uses settled
truncation. DIFFERS comments preserve the history of obsolete ceil/fallback
expectations. Core fixtures include four sides times eleven height ledgers,
raw range/query/bounds/tails, nonsquare foundation records and border order,
zero/equal-fallback/wrapped-key lease chronology, all cell orientations,
first/later zoom and SetTexture, and section 8.2 state/combiner/unsigned-alpha/
pass/force/partial-prefix counterexamples. Expectations are public literals
and bit rules, never owner-generated goldens.

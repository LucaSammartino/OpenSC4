# Corrected entry bridge, before the cached ground-line repair

Source: e0a12625ba59c0bf4381cf3891ef1269ef81fa86. Natural and fixture requests
are preserved separately; both use turbo/end_date in the request header.

| Label | Verdict | Ref warnings | Foreign exceptions | Date | Real seconds | DLL fingerprint |
|---|---|---:|---:|---|---:|---|
| inplace26-fixture-2 | clean | 0 | 0 | 2028-01-16 | 11.672 | 7beea849 |
| inplace26-natural-2 | clean | 0 | 0 | 2028-01-01 | 11.000 | 6dc19786 |

Both DLLs have identical implementation source; the intervening required gate
relinked the DLL. Gate: cargo 7839/0, clippy zero diagnostics, CTest 115/115.
Native machine bridge: 7 cases, 8 dispatches, exact=1, mechanics failures=0.
Six direct-versus-hook controls cover callee cleanup 4/8/12/16 and cdecl 8/12;
the seventh nests an entry and checks expected volatile registers and carry.
x87 data/control are compared without instruction/data pointers. Observer state
is deliberately clobbered before and after native execution. No game code is
used in this synthetic test. The earlier direct-control failure was an authored
test's unspecified-width POP; explicit DWORD operands fixed it.

Natural counters: 155248 calls, 8312 samples, 8305 comparisons, 8252 exact,
53 boolean differences, 6 outside-change samples, 1 refused alias. Float-only=0.
Fixture counters: 512 calls/comparisons/exact, 18 one-value mutations detected
and located in 18 functions. Includes nested calls; top-level fixtures are 16
per installed function. The three naturally unvisited entries are reached by
fixtures (view-viewport/world-viewport; viewport-world is naturally reached here).

Across natural-2 and fixture-2's fixture source, 14/20 functions have only exact
comparisons and no outside signal (2423 body bytes). Requiring no refused call
leaves 13/20 (2372 bytes). Outside signals affect 3/20, attribution unknown.
Rotation/visible-edges remain unsupported by the decoder, not ABI-unknown.

The remaining view-box boolean difference is explained by a missing adapter
input: cached_shadow.rs left both ground lines at their zero default although
Frustum::contains_box reads them. a91f41851 repairs that missing state using
the corner relationship in spec section 3.2; its own scalar word ordering is
unpublished, so the derivation is marked SPEC-GAP. These pre-repair counts are
never relabelled as measurements of that repair.

CSV counters are checked against final log summaries by verify-observations.py.
natural-2-outside.csv contains the five logged locations for six signals (the
logger caps each function at three locations). Region zero contains the owner,
but also unrelated allocator data; neither object identity nor writer is known.

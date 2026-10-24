# Custom forwarding window: `cGZWinCustom`

GOG 1.1.641, *[compat]*. Inherits construction, reference counting, parent/child management and
shutdown from [widget-toolkit.md](widget-toolkit.md). Unlike a generated window, this is a simple
parent-message forwarding surface; see [widget-generator.md](widget-generator.md) for procedures.

## 1. Inventory and identity

The named owner has QueryInterface (`0x0095B9C1`), DoMessage (`0x0095B9F8`), GZPaint
(`0x0095BA43`), and a deleting destructor (`0x0095BA97`). These implement primary window slots
0, 3, 88 and 148 respectively. Slots 130/131 use folded key-down/key-up handlers
(`0x009CD75C`, `0x009CD789`). The deleting destructor delegates to the nondeleting teardown helper
at `0x0095B9A1`, then conditionally frees the allocation. The catch-row inventory includes folded
helpers; it is not evidence of extra independently configurable state.

QI accepts the custom interface IID `0x678D1E84`, returns the secondary pointer at window `+0xD8`
and AddRefs the window. With a null output argument it returns false rather than writing through it.
Other IIDs delegate to base QI. No custom constructor defaults beyond the inherited state are asserted
here; the remaining constructor/secondary-interface attribution is WTC1.

## 2. Forwarding and drawing

`DoMessage` does **not** run the base message dispatcher or generator procedure chain. It forwards the
same message record to the parent window's slot 3, returning that parent's handled result. Without a
parent it returns false. For types **7, 10 and 13 only**, it first adds this window's left and top coordinates
to data1 and data2 **in the caller's record**. No restoration follows forwarding. The adjustment happens
even when there is no parent. This narrow set must not be generalized to all mouse messages.
The comparisons, mutations and parent branch are instruction-verified at `0x0095B9FF..0x0095BA3C`.

Paint sets the inherited draw context's color from the window fill-color query and fills the inherited
draw rectangle, then returns true (`0x0095BA43`). It does not recursively draw children or present the
screen; Plot/Composite/Present retain those responsibilities. This path assumes a usable draw context;
no checked null-context failure was observed.

Direct calls to the folded key handlers have a different target from DoMessage: they send a fresh
record to the notification target at `+0x4C`, with type **5** for key down or **6** for key up, the key
and modifier arguments in data1/data2, and zero data3. They return the target's result. Neither checks
for a null notification target (`0x009CD762..0x009CD786`, `0x009CD78F..0x009CD7B3`). This direct
handler contract does not introduce a base-dispatch fallback into the overriding DoMessage path.

## 3. Conformance vectors and coverage

| Setup/action | Observation |
|---|---|
| Window at `(20,30)`, type 7 with data `(2,3)` | Parent sees `(22,33)`; caller's record retains `(22,33)` afterward. |
| Same input with type 8 | Coordinates stay `(2,3)`. |
| Type 13 with no parent | Coordinates are still adjusted; return false. |
| Parent returns false | Return false directly, without base fallback. |
| Custom IID with null output | False; no AddRef. |
| Direct key-up handler with a valid notification target | Target receives a fresh type-6 record; original caller arguments are not a message record to mutate. |

Behavior for these entry points is specified. Keep the owner partial until WTC1 settles construction,
secondary interface and teardown attribution; do not infer complete closure from its small byte count.

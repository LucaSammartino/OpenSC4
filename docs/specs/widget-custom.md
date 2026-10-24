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
Other IIDs delegate to base QI.

The constructor is the unnamed helper `0x0095BAB3`, not the teardown helper `0x0095B9A1`.
It constructs the base window and installs primary table `0x00AD6AA0` and secondary table
`0x00AD6A8C` at `+0xD8`; it adds no extra scalar defaults or owned resources. These stores
are instruction-verified at `0x0095BAB6..0x0095BACD`. The secondary interface has four slots:

| Slot | Entry | Contract |
|---|---|---|
| 0 | `0x0095BA76` | QI; subtract `0xD8`, then delegate to primary QI. |
| 1 | `0x0095BA81` | AddRef; subtract `0xD8`, then use shared base reference count. |
| 2 | `0x0095BA8C` | Release; same adjustment and shared lifetime. |
| 3 | `0x0095BA6F` | AsIGZWin; return the complete window pointer without AddRef. |

The four entries are confirmed in executable table `0x00AD6A8C..0x00AD6A98` and the
adjustments at their entry instructions. Exported thunk names suggesting a `228`-byte displacement
are misleading: the instructions use **216** bytes (`0xD8`). The next zero word is not a fifth method.

Nondeleting teardown reinstalls these two custom tables, calls **base** Shutdown directly, then
base destruction (`0x0095B9A4..0x0095B9BC`). It neither forwards a final message to the parent
nor owns a custom callback object to release. Primary Shutdown remains inherited. The deleting
destructor at `0x0095BA97` invokes this teardown and frees storage only when its flag's bit 0 is set.
Allocation failure policy belongs to the factory, not to this constructor.

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
| QI custom interface, then AsIGZWin | One QI AddRef; AsIGZWin returns the original complete pointer without another reference. |
| Release through either interface | Both use the same base reference count. |
| Construct and destroy without Init | No custom resources; inherited Shutdown/destruction rules apply. |
| Nondeleting teardown with a parent | No forwarded terminal message; base child/reference cleanup applies. |

The named owner, folded key handlers, constructor and teardown are covered. This closes WTC1;
inherited base-window questions remain in the base spec rather than adding custom behavior.

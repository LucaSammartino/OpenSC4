# Compiled system-service pointer utility

## 1. Evidence and scope

This is the first dedicated contract for original Windows `cRZSysServPtr`
utility bodies, separate from the capped file, INI, directory, stream, command
line and language owners. It specifies normal, readable, nonreentrant calls
whose foreign methods return normally. **[compat]** marks original behavior;
**[choice]** marks replacement design freedom. No original program was executed.

Primary evidence: canonical Windows export queried with `re_query.py func` and
`asm --bytes`. Export identity: `SimCity 4.exe`, image base `0x00400000`, SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`
(`export-summary.txt`). All addresses below are original virtual addresses.
Template names are discovery labels, not proof of body identity. The provisional
43 named entries / 2,247 metadata bytes are neither coverage nor unique utility
byte credit. Foreign framework/service implementations are outside this owner.

## 2. Ordinary stored-pointer construction

**[compat]** Verified bodies at `0x004302E0` (SndSys), `0x00456A30`
(Registry), `0x00449690` (MessageServer2), `0x004496C0` (Persist),
`0x00449720` (Draw) and `0x008FA948` (WWW) operate on a single pointer-sized
field at object offset zero. There is no wrapper vtable written by these bodies.
The accessed field is exactly 32 bits; this is not proof that every nominal
template instance has the same complete object layout.

Construction first clears that field. It reads the framework pointer through
`0x008793EC`, whose complete body loads the 32-bit word at `0x00B540AC` and
returns it. This is a borrowed framework access: the utility makes no framework
AddRef or Release call. If the framework is null, construction stops with a null
field. Otherwise it calls framework virtual offset `+0x14` with two literal
32-bit identity words and the address of the field as output. The constructor
does not test either AL or EAX after that call. It returns the wrapper address
in EAX. ECX supplies the wrapper on entry; there are no stack arguments.

For example, SndSys clears at `0x004302E3`, tests the framework at
`0x004302EE`, supplies output and literals at `0x004302F4..0x004302FA`,
calls at `0x00430301`, and returns the wrapper at `0x00430304`.
WWW uses a shorter three-byte zeroing instruction at `0x008FA94B`, but the
same normal algorithm. No direct service AddRef, QueryInterface, retry or
failure cleanup occurs in these constructors.

Consequently a foreign lookup that returns false but writes a valid nonnull
pointer still leaves that pointer stored. A lookup returning true with null
output leaves the wrapper null. These are utility branch consequences, not a
claim that the original framework produces both combinations. Output not written
remains the initial null. Foreign reference acquisition is not inferred merely
from the boolean return.

## 3. Destruction and reference order

### 3.1 Conditional release without clearing

**[compat]** SndSys `0x00430310` and Registry `0x00456A60` inspect the
32-bit field and do nothing to a null field. Otherwise they call the stored
interface's virtual offset `+0x08` once, with that interface in ECX. There is
no framework acquisition, no AddRef, no field clear and no stored replacement.
The numeric return of the foreign call is ignored. SndSys's decisive addresses
are `0x0043032F..0x0043033F`; Registry's are `0x00456A7F..0x00456A8F`.
WWW `0x008FA96D` has the same normal release rule at
`0x008FA97E..0x008FA98B`, despite its different exception-frame prologue.
These are no-stack-argument member calls. Their return value is not specified
as a usable pointer or reference count. The exception scaffolding does not
establish a public unwind or exception guarantee.

### 3.2 Clear before release body

**[compat]** The body labeled Persist destructor at `0x00874DA3` is different:
it captures the pointer, tests null, clears the field **before** invoking that
pointer's virtual offset `+0x08`, then returns the wrapper address in EAX.
Evidence: load/test `0x00874DA6..0x00874DAA`, clear `0x00874DAC`, call
`0x00874DB1`, return-address setup `0x00874DB4`. A null entry makes no
foreign call. This body is a clear-and-release operation on its ordinary path;
its auto-applied destructor name alone does not certify its source-level role.

### 3.3 Assignment (SndSys body)

**[compat]** At `0x004F2020`, the incoming stack argument is a raw interface
pointer, not the address of another wrapper. If nonnull, its virtual `+0x04`
is called first. Then, if the old field is nonnull, its virtual `+0x08` is
called. Only after those calls does the field receive the new pointer. There
is no pointer-equality shortcut: same-pointer assignment performs both calls
in that order. Null replacement releases the old pointer and then stores null.
Old null/new null performs neither call. Foreign numeric reference returns
are ignored. ECX is the destination wrapper; EAX returns that wrapper; the
callee removes one four-byte argument.

Evidence: argument load `0x004F2046`, new-pointer test/call
`0x004F2049..0x004F2051`, old-pointer test/call
`0x004F2054..0x004F2064`, final store `0x004F207C`, wrapper return
`0x004F207E`, `ret 4` at `0x004F2090`. The export splits the continuation
at `0x004F2075` into a separate function label; it is reached by both normal
branches of this body. Counting only the named 73-byte fragment misses the
store and return. During both foreign calls the wrapper still contains its old
pointer. Reentrant mutation and abnormal foreign exits are outside this rule.

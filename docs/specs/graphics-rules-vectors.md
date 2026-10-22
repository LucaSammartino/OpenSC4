# Graphics rules: independent vectors

Companion to [graphics-rules.md](graphics-rules.md). These are proof-derived fixtures, not executed native
or shadow captures. Each starts fresh unless repeated operations are specified. Trace every setter call.
For script tests use a **fixture-only registry**: `forceSoftwareDriver` bool index 0, `TestBool` bool index 1,
`TestInt` int index 0, `TestFloat` float index 0. These test names do not describe the production registry.
Initially properties, flags and eleven numbers are zero; strings are empty, except stated overrides.

## V1. Integer thresholds and typed write order

Input memory=2.5 (binary32):

```text
partialRule "lower"
  atLeast memory 2.9
  property TestFloat 1.25
  property TestInt 7
  property TestBool true
  property TestInt 9
end
partialRule "upper"
  atMost memory 2.9
  property TestBool false
end
```

Expected successful parse, thresholds both signed integer 2, applied names `lower`; trace bool(1,true),
int(0,7), int(0,9), float(0,1.25). With memory=2 both rules apply, appending bool(1,false).
At the condition-record boundary, threshold text -2.9 stores -2; `atLeast` fails for memory=-2.5.
Threshold 2147483648 stores -2147483648 and `atLeast` passes for memory=0. A NaN environment passes
either numeric condition in all mode, but neither supplies a match in any mode. Evidence: §4.2's
conversion/status branches and §1's dispatch loop.

## V2. Flags aggregate; empty any never matches

```text
partialRule "last flag wins" -any
  has MMX
  hasNo MMX
  property TestInt 1
end
partialRule "empty any" -any
  property TestInt 2
end
rule "empty all"
  property TestInt 3
end
```

Flags clear: names `last flag wins`, `empty all`; int writes 1,3. MMX set: only `empty all`, writing 3.
First rule mask=8, expected=0: it does not retain two contradictory conditions. Evidence: `007E3ED0`,
`007E3F90`, `007CEA8B–007CEA97`.

## V3. Wildcards

For each row make one rule with `stringMatch CPU "<pattern>"` and `property TestBool true`.
The write occurs iff the row matches; replacing the command with `stringNotMatch` inverts the outcome.
CPU and pattern storage retain their original case.

| CPU input | Pattern | Match |
|---|---|---|
| `GeForce FX` | `gEfOrCe*` | true |
| `GeForce FX` | `force` | false |
| `GeForce FX` | `*force*` | true |
| `AB` | `a?` | true |
| `A` | `a?` | false |
| empty | `*` | true |
| empty | `?` | false |
| `a/b` | `a*b` | true |
| `a` | `[a]` | false |

`has mmx` is separately an enum parse error. Evidence: `00918F08–00918F18`, `00918103–0091814D`.

## V4. Nested parse versus evaluation order

```text
rule "parent"
  property TestInt 1
  rule "child"
    has MMX
    property TestInt 8
  end
  rule "child alternative"
    property TestInt 9
  end
  property TestInt 4
end
rule "parent alternative"
  property TestInt 99
end
```

MMX set: names parent, child; int trace 1,4,8. MMX clear: parent, child alternative; trace 1,4,9.
The action after the child blocks still belongs to parent and runs before the child. Add `has FPU` to
parent with FPU clear: only parent alternative applies, writing 99. Evidence: `007E3DD0`, `007E4250`, `007D10B0`.

## V5. Recommendation is not application

```text
optionGroup 2
  option 3
    property TestInt 7
end
partialRule "recommend"
  option 2 3
  option 5 99
end
```

After parse: three groups; group 2 has four options, 0–2 empty; all recommendations 0. Evaluation grows
to six groups; auto(2)=3, auto(5)=99, auto(99)=0; no property writes. Explicit apply(2,3) writes int(0,7).
Apply(2,0), (5,99), (99,0), or unsigned (-1,0) writes nothing and leaves recommendations unchanged.
Reopen group 2/option 3 and append `property TestInt 8`: its next application writes 7 then 8.
Evidence: `007E54C0`, `007E4910`, `007CE870`, `007CE8D0`, `007D10B0`.

## V6. Persistent hardware clearing

With hardwareDriver initially set, evaluate four sibling partial rules: `force` has hardwareDriver and
sets forceSoftwareDriver true; `undo` unconditionally sets it false; `hw` has hardwareDriver and sets
TestInt=8; `sw` hasNo hardwareDriver and sets TestInt=9. All other conditions are absent.
First evaluation: force, undo, sw; bool trace true,false, then int 9. Second evaluation without a gather:
undo, sw; bool false, int 9. Flag 0 stays clear despite the false assignment. A gather with hardware record
byte +0x30 nonzero and force property false restores it. Evidence: §§2.1/5.

## V7. Provider conversion

52-byte record initially zero except +0x20 byte=2, +0x24 dword=3, +0x2C byte=7, +0x30 byte=1. CPU features
contain bits 23 and 0; backing store absent; force property false; probe returns true. Expected flags=0x5F,
textureStages=3.0, both bogo values=+0.0. Set force property true: flags=0x5E, with probe still called.
Backing-store presence adds 0x20. Capability bytes need not equal one.
For the probe boundary itself, hardware byte zero returns false before rendering; hardware true with
no readback returns false. Readback grids with 0,1,2 nonblack RGB samples yield false,false,true;
alpha-only samples do not count. These are outcome fixtures, not hardware render captures.

Total physical bytes=257×2^20+123; available=129×2^20-1; GS video bytes=3×2^20+999; GS texture bytes=2^20-1.
Expected memory=260.0, freeMemory=128.0, videoMemory=3.0, textureMemory=0.0. Description fields
`header`, `SGL`, `1`, `Driver`, `2`, `Card`, `3`, separated by byte 0x0A, produce strings 6–11
SGL,1,Driver,2,Card,3. Evidence: §2.1, `0089159C–008915B9`, `007CFAF7–007CFCF5`.

## V8. Fallback

Force parse failure and evaluate with a registry resolving the four exact fallback property names.

| hardwareDriver | textureStages | Applied names | Clouds/Fog, Shadows, Decals, SecondStage |
|---|---:|---|---|
| clear | 0 | standard software | false,false,false,false |
| clear | 8 | standard software | false,false,false,false |
| set | 1 | standard hardware | true,true,true,false |
| set | 2 | standard hardware; two stages | true,true,true,true |

Last row traces SecondStage false then true. Fallback creates no groups. Evidence: §7.

## V9. Vendor latch

Inject detected vendor=0x1234, device=0x5678, cardName=`Unlisted`. For fixed-offset extraction separately,
use a 27-byte cardVersion filled with `x` except positions 8–11=`1234`, 22–25=`5678`.

```text
vendor "First" 0x1234
  card 0x5678 "Chosen"
  card 0x5678 "Too late"
end
vendor "Second" 0x1234
  card 0x5678 "Also too late"
end
```

Identity becomes `First, Chosen`. Change both first-block devices to 0x5679: identity becomes
`Second, Also too late`. No matching card leaves `Unlisted`. A 26-byte cardVersion leaves detected IDs
zero. Evidence: §§3/4.4, `007E484A–007E48F8`.

## V10. Explicit logging and failures

Initialize a rule that would write TestInt=7. Change the injected environment, then call
LogConfigurationToFile with an explicit path: successful open returns true, regathers and writes only
environment data, closes, with no property writes or evaluation. Failed open returns false but still
regathers. `writeLog:off` changes neither result. SetOptionsFromConfiguration with that switch still
evaluates, writing 7, without another gather. Evidence: `007D0420`, `007D18E0`.

Inside a rule, `property noSuchThing 1` fails with `Unknown render property`; `has mmx` fails enum parsing;
`atLeast memory "2x"` fails with `Bad real number`. Partially parsed lists are not published. Init installs
fallback on failure and has no parse-success return. Evidence: `007E4250`, `0067BBCE`, `007D18C2–007D18CD`.

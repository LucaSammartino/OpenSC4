# glue18-repl-9

The terrain replacement (`enable = terrain` + `terrain_shadow = 1`) at the end
of the lane. Turbo to 2032-01-01 on Konradshohe.

```
AUTOTEST VERDICT: corrupt (foreign_exceptions=1, watchdog_stalls=0, rtc=0,
  overruns=0, ref_warnings=0, julian=0)
```

**Not clean.** The city now builds and reaches `PostCityInit` with our terrain,
but it faults before the run can start. `ref_warnings=0` in every attempt.

## What the run does reach

```
terrain: Read (class record = version + ref, §13.2) -> ok=1 version=2
terrain: created cSTESurfaceWater (a view over the shared fc4_terrain handle)
terrain: replacement ON (enable = terrain): 128 x 128 cells, sea=250, default brush level=0
observer: PostCityInit (replacement)
autotest: city loaded (PostCityInit); clock starts at the next tick
terrain: created the terrain 3D view (class 0xC9B84E10) at DoPostCityInit
```

The class record reads correctly (version 2 + the object reference), both
classes are built by the city, the core is created for Konradshohe's 128x128
grid with the shipped sea level, and the city load completes. `ref_warnings=0`
throughout: no reference is released that was not taken.

## What is left

After `DoPostCityInit` the game calls into the terrain's 3D view (class
`0xC9B84E10`, `terrain.md` §2.3 `+0xE8`, slot 66). The class factory creates
the object (the log line above), but its own initialisation contract is not in
`terrain.md` - "Drawing (textures, cliffs as art, waves, decals, steam,
lighting, the 3D terrain view) is out of scope" - and a half-initialised view
is then called through a wrong slot (`eip=0x41800000`, the float 16.0, which is
what a data word looks like when it is taken for a code pointer).

The two earlier crashes on the way, both fixed, are in
[docs/lanes/glue18.md](../lanes/glue18.md).

## Earlier attempts (same request, different DLL)

| label | fault | cause |
|---|---|---|
| glue18-repl-1 | write to 0 in `AddParticle` (0x005B9FEB) | the city built the ORIGINAL `cSTESurfaceWater` (its CLSID was not registered) and it ran against a null terrain |
| glue18-repl-2 / 3 / 4 | read of 0 in `cSC4City::Init` (0x0045BF05), `ecx=0` | the component stream was desynchronised: `Read` consumed the u16 version but not the object reference (foundations §2.7); then the surface water's IID `0xAB8DA759` was refused |
| glue18-repl-6 | same | the object reference was still not consumed |
| glue18-repl-7 / 8 | read of 0 in `cSC4ZoneDeveloper::DoMessage` (0x0073291D) | the zone developer's `PostCityInit` handler takes the terrain view from slot 66 (zones.md §5.2) and dereferenced the null one |
| glue18-repl-9 | jump to `0x41800000` | the view is created but not initialised |

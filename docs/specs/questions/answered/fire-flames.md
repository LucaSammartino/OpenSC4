# fire-flames: answered questions

<!-- from: core-fire-flames4 (2026-10-22) -->
~~- **FF4 - §1 transform and §3 box ordering:** please publish the positive-y rotation matrix signs, trig precision, transform mask/counter writes, and exact side placement draw order (free horizontal coordinate versus height). Are top/flora wall and side flags inherited from construction, and is visibility cleared when hiding or only the effect stopped? [standard right-handed positive-y rotation, f64 trig stored as f32; semantic transform uses the existing Xform, without a native counter claim; free coordinate then height; retain explicit initial flags unless assigned by the spec; hiding clears visible and stops an existing effect]~~

**ANSWERED 2026-10-22 (re-small-renderer-contracts4):** DIFFERS — §§4.1–4.2: matrix rows (c,0,s),(0,1,0),(−s,0,c), unsigned side×90 radians with x87 trig and stated float stores; temporary counter=2/kind=5, embedded transform unchanged; side then horizontal then height, explicit wall writes and inherited flora flags; hiding clears visible. ViewUpdate sends old-position then new-position transforms.

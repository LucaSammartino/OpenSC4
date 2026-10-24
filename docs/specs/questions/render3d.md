# render3d: open questions
<!-- Questions about [render3d.md](../render3d.md). Append new entries at the END; when one is answered, move it to answered/render3d.md. Rules: README.md. -->

- **RND1 - section 2:** the name of cIGZOStream/cIGZIStream slot 8 (`+0x20`) used for the viewport fields and the
  camera mode in `cSC43DRender::Write`/`Read` (`0x007C310B`, `0x007C31AD`): `PutSint32`/`GetSint32` is inferred from
  slot 9 being `PutUint32`. Settle from the stream vtable row in docs/abi/interfaces.tsv.
- **RND4 - section 10.1:** the `StartScreenShake` overload (slot 43, `0x007C6BC0`) never writes `P+0x04`, the
  fade-out length copied to node `+0x20` (stack slot frame+4 is not stored between `0x007C6BC0` and the slot-42 call at
  `0x007C6D4C`), so `StopScreenShake` on such a shake sees stack garbage. The spec has a replacement store 0.0
  (stop at once). Confirm by a game run whether stopping an overload-started shake ends it immediately or fades it.

# auto-game-pause: open questions

- **AGP1 — auto-game-pause §3.1:** What is the intended typed interface and semantic operation name for F1? The matching Windows 1.1.641 PE/export literally requests framework service `0xC416025C` with IID `0x0073283C`, then calls no-argument slots `+0x48` / `+0x4C`. The public ID table labels the service graphic system; the familiar sound label is not established by these instructions. Do not substitute a guessed sound IID or direct native target. The raw lookup/slot/Release contract is known; typed semantics remain open.

# Spec: standalone string encoding conversion

## 1. Scope, evidence and ownership

Windows 1.1.641 instruction evidence; no native execution or implementation output.
This is the first dedicated whole-converter research round, **not** the first research
on every instruction in it. The earlier `re-strings4` round-2 report explicitly left
ConvertStringEncoding unstarted after Utility/Manager. The inherited final storage
owner excludes this separate converter. FS16 already specifies a **partial** UTF-8
to UTF-16 path used by MakeLower: [graphics-formats §3.11](graphics-formats.md#311-regp-12-and-later-compat),
specifically its FS16 discussion. Its low-16-bit, no-surrogate, invalid-input and
truncation facts remain canonical prior findings. String storage, language startup,
UTF-8 validation, Thai, font and TextEdit owners are outside this round.

Addresses below cite original instructions, queried from the matching Windows export
with `re_query.py asm --bytes`; decompilation is navigation only. The six candidate
metadata sizes total 2,159 bytes; that total is not proof of six independent bodies
or complete coverage. The per-entry ledger below determines the actual scope.

## 2. Raw counted interface and dispatch

The core at `00910E18` has six stack arguments in this order: source address,
32-bit source count `N`, source selector, destination address, address of a 32-bit
capacity/result count `C`, destination selector. Its return is a full 32-bit EAX
count, not an AL boolean. Counts are source/destination encoding units: bytes except
for selector `10` hexadecimal, whose units are little-endian 16-bit words.

For `N = 0`, store zero in `C`, return zero, and do not touch destination or resolve
selectors (`00910E1F–00910E2D`). Otherwise resolve source first, then destination:
`FFFFFFFE` becomes `8`; `FFFFFFFF` calls `GetCurrentSystemCodePage` (`00910014`)
separately for that occurrence. Other values pass through. Selector `8` means this
converter's UTF-8, selector `10` hexadecimal means its UTF-16 word representation.
Treat system-codepage resolution as an existing raw primitive, not a fixed 1252
constant or a request to reimplement locale startup.

Before testing equality, scan **all N source bytes** if the resolved source is not
`10`. If every byte is at most `7F`, replace the source selector with `0`
(`00910E6A–00910E7F`). Embedded zero is an ordinary counted byte. There is no
sentinel-length scan in this core; `FFFFFFFF` in the count slot is not a supported
request for strlen.

Dispatch order (`00910E81–00910EAF`, `00911258–00911323`):

1. Equal selectors, source `0` to any non-`10` target, or any non-`10` source to
   target `0`: raw copy path. Copy `min(N,C)` units; double the byte size only for
   target `10`. Write that unit count to `C` and return it. No terminator is added.
2. Source `10` to target `0`: copy the low byte of each of `min(N,C)` source words;
   write/return that count (`00910EB3–00910EDA`). No range validation.
3. Remaining distinct source/destination addresses dispatch to the conversion paths.
   A same-address conversion reaching `00910EA4` returns zero **without changing C**.
   Alias, overlap and in-place operation are otherwise outside the supported domain.
4. Source `0` to target `10`: widen each counted byte to one word, at most `min(N,C)`;
   write/return that count (`00911260–00911289`).

The raw copy path uses memcpy and does not validate equal-selector byte forms.
This interface has no proven null-output size-query mode: the owned loops write
directly and the copy path calls memcpy. Capacity zero is handled by each branch,
not by a common sizing pass.

## 3. Conversion branches

Further branch and wrapper contracts follow in this dedicated spec.

## 4. Proposed literal evidence ledger

All vectors here are **PROPOSED / UNEXECUTED**, derived from instructions rather
than a standard decoder or implementation result. Distinct source/destination
buffers are assumed. Destination bytes not explicitly changed keep their sentinel.

| ID | Raw input; N; selectors source → target | Initial destination; C | Expected destination; C; EAX; calls |
|---|---|---|---|
| SE01 | `41 00 42`; 3; `1252 → 8` | `CC CC CC CC`; 2 | `41 00 CC CC`; 2; 2; memcpy(2), no OS conversion (ASCII shortcut) |
| SE02 | `E9 41`; 2; `1252 → 1252` | `CC CC CC CC`; 1 | `E9 CC CC CC`; 1; 1; memcpy(1), no OS conversion |
| SE03 | words `12E9 0042`; 2; `10 → 0` | `CC CC CC`; 1 | `E9 CC CC`; 1; 1; no OS conversion |
| SE04 | `7F 00 41`; 3; `8 → 10` | words `CCCC CCCC CCCC CCCC`; 3 | words `007F 0000 0041 CCCC`; 3; 3; no OS conversion (widen path) |
| SE05 | empty; 0; `FFFFFFFF → FFFFFFFF` | `CC CC`; 7 | `CC CC`; 0; 0; no codepage-resolution or conversion calls |

## 5. Attribution ledger

Pending boundary and wrapper audit; no full-byte credit claimed at this milestone.

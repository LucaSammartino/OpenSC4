# Larger candidates examined before the new cohort

No executable, decompilation or tools/ content was consulted. These are binding
or dependency exclusions, not measured numeric mismatches. No questions filed.

| Candidate | Published contract and reason for rejection |
|---|---|
| Sprite SetOrientation, 1978 export bytes | animated-sprite §3 gives a slot/signature, but neither vtables.tsv nor direct-calls.md has this entry and the spec does not publish its calling convention/callee pop. Not guessed. |
| Sprite UpdateView, 1669 export bytes | §5.1 requires camera-controller magnification and optionally camera view volume; it is not closed. No complete entry convention/argument contract in the permitted ABI. |
| Sprite GetTextureData | §4.1 specifies the record decoding but no complete callable convention/stack contract. No matching ABI entry; not guessed. |
| RefPack CompressData | vtables.tsv publishes thiscall/four words/pop16, but not the buffer roles/order, length ownership or output-length protocol. §2.7 describes the stream/decode, not this interface's compression protocol or exact native encoder choices. |
| RefPack DecompressData | Same four-word binding gap; §2.7 additionally specifies unchecked malformed native accesses. No bounded replay until argument roles are known. |
| CalcShadowTerrainRect | lighting §5.6 calls it `(…, box, outRect)`; ABI gives four words/pop16 without identifying the first two. The terrain grid/strides are described, but do not resolve the call. |
| Fullpath(out,rel,max,base) | paths §1.2 gives four parameters but no calling convention or callee pop; no direct-call/virtual ABI row. Null-output allocation and before-buffer underflow are additional domain restrictions. |
| BuildFirstSets | html-document §4.1 explicitly mutates nine global std::set trees at 00b624b8..00b62518, outside receiver/argument windows. No entry ABI; not safely invoked or counted as an observed outside write. |
| HTML AppendSet, IsInSet | Native set-node layout/call ABI absent. AppendSet allocates nodes, which are not a closed fixed buffer. |
| Sprite LookAt, 318 export bytes | Same missing published cleanup/convention as SetOrientation; §3.1 supplies semantics but not a complete entry contract. |

The highest-byte functions with explicit sizes were considered first. Address
spans in semantic traces are not silently converted into function-body sizes.
The resulting smaller cohort is a limited negative/inconclusive experiment,
not an interchangeable substitute for the requested >=8 KB throughput test.

// SPDX-License-Identifier: GPL-3.0-or-later
// Isolated native dependency double for effects.md 18.9.7's worked resource case.
struct MapNode { MapNode* next; Vector key; uint32_t index; };
static_assert(sizeof(MapNode) == 0x14);
void DestroyVisualNames(void* dictionary) {
    auto& buckets = Field<Vector>(dictionary, 4);
    for (auto* p = buckets.begin; p != buckets.end; p += 4) {
        for (auto* node = *reinterpret_cast<MapNode**>(p); node;) {
            auto* next = node->next;
            FreeVector(node->key); Free(node); node = next;
        }
    }
    FreeVector(buckets);
}
MapNode* MakeNode(const char* text, uint32_t length, uint32_t index) {
    auto* node = static_cast<MapNode*>(Allocate(sizeof(MapNode)));
    auto* key = static_cast<uint8_t*>(Allocate(length + 1));
    std::memcpy(key, text, length); key[length] = 0;
    *node = {nullptr, {key, key + length, key + length + 1}, index};
    return node;
}
void* expectedDictionary = nullptr;
unsigned growCalls = 0, insertCalls = 0;
void GrowNames(void* dictionary, uint32_t count) {
    if (dictionary != expectedDictionary || count != Field<uint32_t>(dictionary, 0x10) + 1 ||
        growCalls != insertCalls) throw std::runtime_error("wrong growth receiver/value/order");
    ++growCalls; // C1/C3's capacities 2, 3, 4 require no growth from 193 buckets.
}
FC4EffectsVisualIterator* InsertName(void* dictionary, FC4EffectsVisualIterator* out, const FC4EffectsVisualName* pair) {
    if (dictionary != expectedDictionary || growCalls != insertCalls + 1 ||
        pair->end - pair->begin != 5 || std::memcmp(pair->begin, "MiXeD", 5))
        throw std::runtime_error("wrong insert receiver/result/pair/order");
    auto* node = MakeNode(pair->begin, 5, pair->index);
    auto** head = reinterpret_cast<MapNode**>(Field<Vector>(dictionary, 4).begin) + 140;
    // Only MiXeD is submitted by this fixture. Insert immediately after its first match.
    if (*head) { node->next = (*head)->next; (*head)->next = node; }
    else *head = node;
    ++Field<uint32_t>(dictionary, 0x10); ++insertCalls;
    *out = {node, dictionary};
    return out;
}
int VisualMapWorkedCase() {
    auto dependencies = bindings;
    dependencies.growVisualNames = GrowNames;
    dependencies.insertVisualName = InsertName;
    dependencies.readDescription = [](uint32_t family, uint16_t version, cIGZIStream* stream, void* p) {
        if (family != 2 || version != 2) throw std::runtime_error("wrong visual dependency");
        stream->GetUint32(Field<uint32_t>(p, 0)); // stand-in for a decoded renderer record
    };
    dependencies.writeDescription = [](uint32_t family, cIGZOStream* stream, const void* p) {
        if (family != 2) throw std::runtime_error("wrong visual writer");
        stream->SetUint32(*static_cast<const uint32_t*>(p));
    };
    auto* resource = static_cast<FC4IEffectsResource*>(CreateFC4EffectsResourceWithBindings(dependencies));
    CHECK(resource && resource->AddRef() == 1);
    auto* collection = reinterpret_cast<uint8_t*>(resource) + 4;
    expectedDictionary = collection + 4;
    auto& buckets = Field<Vector>(expectedDictionary, 4);
    buckets.begin = static_cast<uint8_t*>(Allocate(193 * 4));
    std::memset(buckets.begin, 0, 193 * 4);
    buckets.end = buckets.capacity = buckets.begin + 193 * 4;
    auto* old = MakeNode("old", 3, 4);
    auto** heads = reinterpret_cast<MapNode**>(buckets.begin);
    heads[134] = old;
    Field<uint32_t>(expectedDictionary, 0x10) = 1;
    Stream empty; CHECK(resource->Write(&empty));
    Stream prefix;
    prefix.bytes.assign(empty.bytes.begin(), empty.bytes.begin() + 0x3E);
    prefix.bytes[0x3A] = 10; // ten visual records, decoded through the injected dependency
    for (uint32_t i = 0; i < 10; ++i) prefix.Put(i);
    Stream input; input.bytes = prefix.bytes;
    for (uint32_t i : {7, 9}) { input.SetRZCharStr("MiXeD"); input.Put(i); }
    input.SetRZCharStr("end"); input.Put<uint32_t>(0xFFFFFFFF);
    input.Put<uint8_t>(0); input.Put<uint32_t>(0); input.Put<uint16_t>(0); input.Put<uint32_t>(0);
    input.status = false; // normal false primitive statuses are ignored
    CHECK(resource->Read(&input) && input.cursor == input.bytes.size());
    CHECK(growCalls == 2 && insertCalls == 2 && Field<uint32_t>(expectedDictionary, 0x10) == 3);
    CHECK(heads[134] == old && old->index == 4 && !old->next);
    auto* a = heads[140]; auto* b = a->next;
    CHECK(a->index == 7 && b && b->index == 9 && !b->next && a->key.begin != b->key.begin);
    CHECK(std::memcmp(a->key.begin, "MiXeD\0", 6) == 0 && a->key.end - a->key.begin == 5);
    Stream expected; expected.bytes = prefix.bytes;
    expected.SetRZCharStr("old"); expected.Put<uint32_t>(4);
    for (uint32_t i : {7, 9}) { expected.SetRZCharStr("MiXeD"); expected.Put(i); }
    expected.SetRZCharStr("end"); expected.Put<uint32_t>(0xFFFFFFFF);
    expected.Put<uint8_t>(0); expected.Put<uint32_t>(0); expected.Put<uint16_t>(0); expected.Put<uint32_t>(0);
    std::fill(input.bytes.begin(), input.bytes.end(), 0); // source lifetime is independent
    Stream output; CHECK(resource->Write(&output) && output.bytes == expected.bytes);
    Stream more; more.bytes = prefix.bytes;
    more.SetRZCharStr("MiXeD"); more.Put<uint32_t>(11);
    more.bytes.insert(more.bytes.end(), expected.bytes.end() - 22, expected.bytes.end());
    CHECK(resource->Read(&more) && more.cursor == more.bytes.size());
    CHECK(growCalls == 3 && insertCalls == 3 && Field<uint32_t>(expectedDictionary, 0x10) == 4);
    CHECK(heads[140] == a && a->next->index == 11 && a->next->next == b && !b->next);
    CHECK(resource->Release() == 0);
    expectedDictionary = nullptr;
    return 0;
}

// SPDX-License-Identifier: GPL-3.0-or-later
// Included inside FC4SCResShadow.cpp's anonymous namespace. Only the adapter
// layout and read calls published by sc-resources §§5/6 are exercised here.
// SCR8 still blocks calling the command/file parser or image/writer owners.
class InputRecord final : public cIGZPersistDBSerialRecord {
public:
    InputRecord(const uint8_t* bytes, uint32_t size) : bytes_(bytes), size_(size) {}
    uint32_t unexpected = 0;
    bool QueryInterface(uint32_t, void**) override { ++unexpected; return false; }
    uint32_t AddRef() override { ++unexpected; return 1; }
    uint32_t Release() override { ++unexpected; return 1; }
    cIGZPersistDBRecord* AsIGZPersistDBRecord() override { ++unexpected; return nullptr; }

    // persistence §§3.2/3.3, native order 1, read-only bounded input. Unused
    // operations refuse and count, so unsupported dispatch cannot pass silently.
#define FC4_SCR_INPUT_TYPE(Name, Type) \
    bool GetField##Name(Type& value) override { return GetFieldVoid(&value, sizeof(value)); } \
    bool GetField##Name(Type* value, uint32_t count) override { \
        return count <= UINT32_MAX / sizeof(Type) && GetFieldVoid(value, count * sizeof(Type)); } \
    bool SetField##Name(Type) override { ++unexpected; return false; } \
    bool SetField##Name(Type*, uint32_t) override { ++unexpected; return false; }
    FC4_SCR_INPUT_TYPE(Uint8, uint8_t)
    FC4_SCR_INPUT_TYPE(Sint8, int8_t)
    FC4_SCR_INPUT_TYPE(Uint16, uint16_t)
    FC4_SCR_INPUT_TYPE(Sint16, int16_t)
    FC4_SCR_INPUT_TYPE(Uint32, uint32_t)
    FC4_SCR_INPUT_TYPE(Sint32, int32_t)
    FC4_SCR_INPUT_TYPE(Uint64, uint64_t)
    FC4_SCR_INPUT_TYPE(Sint64, int64_t)
    FC4_SCR_INPUT_TYPE(Float32, float)
    FC4_SCR_INPUT_TYPE(Float64, double)
#undef FC4_SCR_INPUT_TYPE
    bool GetFieldString(cIGZString&) override { ++unexpected; return false; }
    bool SetFieldString(const cIGZString&) override { ++unexpected; return false; }
    bool GetFieldResKey(cGZPersistResourceKey& key) override {
        return GetFieldUint32(key.type) && GetFieldUint32(key.group) && GetFieldUint32(key.instance);
    }
    bool SetFieldResKey(const cGZPersistResourceKey&) override { ++unexpected; return false; }
    bool GetFieldVariant(cIGZVariant**) override { ++unexpected; return false; }
    bool SetFieldVariant(const cIGZVariant*) override { ++unexpected; return false; }
    bool SetFieldVoid(void*, uint32_t) override { ++unexpected; return false; }
    bool GetFieldVoid(void* out, uint32_t count) override {
        if (count > size_ - position_) return false;
        if (count) std::memcpy(out, bytes_ + position_, count);
        position_ += count;
        return true;
    }
    uint32_t GetSize() override { return size_; }
    bool SetSize(uint32_t) override { ++unexpected; return false; }
    uint32_t GetPosition() override { return position_; }
    bool SeekAbsolute(uint32_t position) override {
        if (position > size_) return false;
        position_ = position;
        return true;
    }
    bool SeekRelative(int32_t delta) override {
        const int64_t position = static_cast<int64_t>(position_) + delta;
        return position >= 0 && position <= size_ && SeekAbsolute(static_cast<uint32_t>(position));
    }
private:
    const uint8_t* bytes_;
    uint32_t size_, position_ = 0;
};

struct InputGroup {
    const char* name;
    uint32_t cases = 0, mismatches = 0, refused = 0;
};
InputGroup g_specBinary{"constructed-spec-binary"}, g_specText{"constructed-spec-text"};
InputGroup g_resource{"constructed-resource-prefix"};
bool g_constructedReported = false;

void ConstructedRead(InputGroup& group, unsigned mode, unsigned slot,
                     const uint8_t* bytes, uint32_t length, uint32_t total) {
    // Refuse an unknown executable/table entry before making any direct call.
    auto** table = reinterpret_cast<void**>(kTables[mode]);
    const auto& patch = g_patches[mode][slot];
    if (!compat::IdentifiedVersion() || (table[slot] != reinterpret_cast<void*>(kFunctions[mode][slot]) &&
        (!patch.entry || table[slot] != patch.forward))) {
        ++group.refused;
        return;
    }
    InputRecord record(bytes, length);
    // Literal §6.1 layout, no unknown constructor/destructor call or ownership
    // transfer. This small borrowed adapter is not an engine-owned COM object.
    struct Adapter { void** table; cIGZPersistDBSerialRecord* record; uint32_t counter, total; };
    static_assert(sizeof(Adapter) == 16 && offsetof(Adapter, counter) == 8, "published x86 adapter state");
    Adapter adapter{table, &record, 4, total};
    alignas(8) uint8_t output[16];
    std::memset(output, 0xA5, sizeof(output));
    Observation o{};
    o.text = mode;
    o.slot = slot;
    o.counter = adapter.counter;
    o.total = total;
    std::memcpy(o.before, output, 12);
    using Original = bool (__thiscall*)(void*, void*);
    const auto native = reinterpret_cast<Original>(kFunctions[mode][slot]);
    o.status = native(&adapter, output) ? 1u : 0u;
    o.afterCounter = adapter.counter;
    std::memcpy(o.after, output, 12);
    o.bytes = bytes;
    o.length = length;
    o.consumed = record.GetPosition();
    ++group.cases;
    if (record.unexpected) ++group.refused;
    const auto difference = fc4_scres_shadow_compare(&o);
    if (difference) {
        ++group.mismatches;
        if (group.mismatches <= 8) fc4::Log("scres constructed difference: group=%s case=%u slot=%u channels=%u consumed=%u/%u status=%u counter=%u/%u",
            group.name, group.cases, slot, difference, o.consumed, length, o.status, o.counter, o.afterCounter);
    }
}

void ConstructedSpecInputs() {
    // SCR-V13..15, V26 and §6.2 narrowing/overflow/prefix contracts. These are
    // adapter vectors; no command/expression-parser coverage is claimed.
    static const uint8_t binary[] = {23, 0, 0, 0, 3, 0, 1, 0, 0x12, 0x34, 0x56, 0x78, 0, 0, 0, 0};
    for (unsigned slot = 1; slot != 14; ++slot) {
        if (slot == 3) continue;
        ConstructedRead(g_specBinary, 0, slot, binary, sizeof(binary), sizeof(binary) + 4);
        ConstructedRead(g_specBinary, 0, slot, binary, sizeof(binary), static_cast<uint32_t>(Width(slot)) + 4);
    }
    struct TextCase { unsigned slot; const char* text; };
    static const TextCase cases[] = {
        {1, "[ 23, 3, 1 ]= "}, {1, "name[ 42, 65539, 2 ]= "},
        {1, "[ 23, 3,"}, {2, "1, 2, 3, "}, {2, "1, 2, "},
        {4, "1.25, "}, {4, "1e300, "}, {4, "nan, "},
        {5, "-12.5suffix, "}, {5, "1e40, "}, {6, "-9223372036854775809, "},
        {8, "-257, "}, {9, "12suffix, "}, {9, "suffix, "},
        {9, "4294967296, "}, {10, "65537, "}, {11, "-257, "},
        {12, ";comment\r-257, "}, {13, "256, "}, {9, "42"}, {9, ""}
    };
    for (const auto& c : cases) {
        const auto n = static_cast<uint32_t>(std::strlen(c.text));
        ConstructedRead(g_specText, 1, c.slot, reinterpret_cast<const uint8_t*>(c.text), n, n + 4);
    }
    // Text uint64 deliberately inherits the binary reader (§6.1).
    ConstructedRead(g_specText, 1, 7, binary, sizeof(binary), sizeof(binary) + 4);
}

void ConstructedResourceInputs() {
    // feature-manager §4.1: fixed shipped pack-1 INI key. This tests raw
    // resource bytes through numeric binary adapters, NOT the INI parser.
    cIGZPersistResourceManagerPtr rm;
    if (!rm) { ++g_resource.refused; return; }
    const cGZPersistResourceKey key(0, 0x8A5971C5, 0x2B563701);
    cIGZPersistDBRecord* record = nullptr;
    (void)rm->OpenDBRecord(key, &record, false);
    if (!record) { ++g_resource.refused; return; }
    cIGZPersistDBSerialRecord* serial = nullptr;
    const bool queried = record->QueryInterface(GZIID_cIGZPersistDBSerialRecord, reinterpret_cast<void**>(&serial));
    if (serial) FC4_ACQUIRED(serial, "scres constructed serial QI");
    uint8_t bytes[16]{};
    const bool read = queried && serial && serial->GetSize() >= sizeof(bytes) &&
        serial->SeekAbsolute(0) && serial->GetFieldVoid(bytes, sizeof(bytes));
    if (read) {
        uint32_t hash = 2166136261u;
        for (auto b : bytes) hash = (hash ^ b) * 16777619u;
        fc4::Log("scres input: key=00000000/8A5971C5/2B563701 window=0:16 fnv=%08X partial=adapter_reads", hash);
        for (unsigned slot = 4; slot != 14; ++slot)
            ConstructedRead(g_resource, 0, slot, bytes, sizeof(bytes), sizeof(bytes) + 4);
    } else ++g_resource.refused;
    if (serial) FC4_RELEASE(serial);
    rm->CloseDBRecord(key, &record);
}

void ConstructedLines(const fc4_date& date) {
    if (!g_constructed || g_constructedReported) return;
    g_constructedReported = true;
    ConstructedResourceInputs();
    for (const auto* group : {&g_specBinary, &g_specText, &g_resource}) {
        const auto bad = group->mismatches + group->refused + (group->cases == 0 ? 1u : 0u);
        fc4::Log("scres shadow %04d-%02d-%02d %s: partial=adapter_reads exact=%u/%u refused=%u; mismatches=%u",
            date.year, date.month, date.day_of_month, group->name,
            group->cases - group->mismatches, group->cases, group->refused, bad);
    }
}

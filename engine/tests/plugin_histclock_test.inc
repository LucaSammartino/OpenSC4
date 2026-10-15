// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.
//
// The history warehouse and 24-hour clock replacements and their shadows
// (docs/specs/history.md, clock24.md), included by plugin_load_test.cpp. The
// DLL runs without a game framework: test seam op 54 (FC4HistClock.h)
// answers the game calls instead.

namespace {

constexpr uint32_t kCLSID_History = 0x89EFA536;
constexpr uint32_t kIID_cISC4HistoryWarehouse = 0x69F4A803;
constexpr uint32_t kHcMsgRecord = 0x89EFA536;
constexpr uint32_t kHcMsgQueueLength = 0x89EFA537;
constexpr uint32_t kHcMsgSimNewDay = 0x66956814;
constexpr uint32_t kHcMsgSimNewYear = 0x66956817;
constexpr uint32_t kHcIIDMessageTarget2 = 0x452294AA;
constexpr uint32_t kHcDay0 = 2451545;  // 2000-01-01

// A cIGZDate seen through the compiled slots: only DayNumber (slot 26) is read.
class HcDate final : public cFC4IGZDate {
public:
    explicit HcDate(uint32_t d) : day(d) {}
    uint32_t day;
    bool QueryInterface(uint32_t, void**) override { return false; }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
    bool SetMonthNameDayYear(char const*, uint32_t, uint32_t) override { return false; }
    bool SetDayNumber(uint32_t d) override {
        day = d;
        return true;
    }
    bool SetTime(cRZTime const&) override { return false; }
    bool SetMonthDayYear(uint32_t, uint32_t, uint32_t) override { return false; }
    bool SetDayOfYear(uint32_t, uint32_t) override { return false; }
    bool Copy(cFC4IGZDate const&) override { return false; }
    bool Clone(cFC4IGZDate**) override { return false; }
    bool Between(cFC4IGZDate const&, cFC4IGZDate const&) override { return false; }
    int32_t CompareTo(cFC4IGZDate const&) override { return 0; }
    uint32_t WeekDay() override { return 0; }
    uint32_t Year() override { return 0; }
    uint32_t DayOfYear() override { return 0; }
    uint32_t DayOfMonth() override { return 0; }
    uint32_t FirstDayOfMonthOf(uint32_t) override { return 0; }
    uint32_t FirstDayOfMonth() override { return 0; }
    uint32_t Hash() override { return day; }
    bool IsValid() override { return true; }
    bool Leap() override { return false; }
    bool MaxDate(cFC4IGZDate const&, cFC4IGZDate&) override { return false; }
    bool MinDate(cFC4IGZDate const&, cFC4IGZDate&) override { return false; }
    uint32_t Month() override { return 0; }
    uint32_t Previous(uint32_t, cFC4IGZDate&) override { return 0; }
    uint32_t YearLastTwoDigits() override { return 0; }
    uint32_t DayNumber() override { return day; }
    bool IsLess(cFC4IGZDate const&) override { return false; }
    bool IsLessOrEqual(cFC4IGZDate const&) override { return false; }
    bool IsGreater(cFC4IGZDate const&) override { return false; }
    bool IsGreaterOrEqual(cFC4IGZDate const&) override { return false; }
    bool IsEqual(cFC4IGZDate const&) override { return false; }
    bool IsNotEqual(cFC4IGZDate const&) override { return false; }
    uint32_t MinusDays(int32_t) override { return 0; }
    uint32_t MinusDate(cFC4IGZDate const&) override { return 0; }
    uint32_t PlusDays(int32_t) override { return 0; }
    uint32_t PlusDate(cFC4IGZDate const&) override { return 0; }
    cFC4IGZDate& Increment() override { return *this; }
    cFC4IGZDate& Decrement() override { return *this; }
    cFC4IGZDate& AddDays(int32_t) override { return *this; }
    cFC4IGZDate& SubtractDays(int32_t) override { return *this; }
    bool DateString(cIGZString&, uint32_t) override { return false; }
    bool DayWithinMonth(uint32_t, uint32_t, uint32_t) override { return false; }
    uint32_t DaysInYear(uint32_t) override { return 365; }
    uint32_t Jday(uint32_t, uint32_t, uint32_t) override { return 0; }
    bool LeapYear(uint32_t) override { return false; }
};

// A standard message: type, data1, data2 and void2 (the cheat text).
class HcMessage final : public cIGZMessage2Standard {
public:
    HcMessage(uint32_t t, intptr_t d1, intptr_t d2 = 0, void* v2 = nullptr) : type(t), data1(d1), data2(d2), void2(v2) {}
    uint32_t type;
    intptr_t data1, data2;
    void* void2;
    bool QueryInterface(uint32_t riid, void** p) override {
        if (p == nullptr || riid != GZIID_cIGZMessage2Standard) return false;
        *p = static_cast<cIGZMessage2Standard*>(this);
        return true;
    }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
    bool Create(uint32_t, void**) const override { return false; }
    uint32_t GetType() const override { return type; }
    void SetType(uint32_t t) override { type = t; }
    bool operator==(cIGZMessage2 const&) const override { return false; }
    bool operator<(cIGZMessage2 const&) const override { return false; }
    bool GetHasData1() const override { return true; }
    intptr_t GetData1() const override { return data1; }
    void* GetVoid1() const override { return nullptr; }
    void SetData1(intptr_t v) override { data1 = v; }
    void SetVoid1(void*) override {}
    bool GetHasData2() const override { return true; }
    intptr_t GetData2() const override { return data2; }
    void* GetVoid2() const override { return void2; }
    void SetData2(intptr_t v) override { data2 = v; }
    void SetVoid2(void* v) override { void2 = v; }
    bool GetHasData3() const override { return false; }
    intptr_t GetData3() const override { return 0; }
    void* GetVoid3() const override { return nullptr; }
    void SetData3(intptr_t) override {}
    void SetVoid3(void*) override {}
    bool GetHasData4() const override { return false; }
    intptr_t GetData4() const override { return 0; }
    void* GetVoid4() const override { return nullptr; }
    void SetData4(intptr_t) override {}
    void SetVoid4(void*) override {}
    bool GetHasString() const override { return false; }
    bool GetString(uint32_t, void**) const override { return false; }
    void SetString(cIGZString*) override {}
    bool GetHasIGZUnknown() const override { return false; }
    cIGZUnknown* GetIGZUnknown() const override { return nullptr; }
    void SetIGZUnknown(cIGZUnknown*) override {}
    bool GetHasExtra() const override { return false; }
    uint32_t GetExtra(uint32_t, void**) const override { return 0; }
    void SetExtra(cIGZSerializable*) override {}
};

// What the hooks saw.
struct HcNote {
    cIGZMessageTarget2* target;
    uint32_t type;
    bool add;
};
struct HcAgentCall {
    cIGZMessageTarget2* target;
    uint32_t level;
    std::string name;
    uint32_t flags;
    bool add;
};
struct HcWorld {
    uint32_t date = kHcDay0;
    std::vector<HcNote> notes;
    std::vector<HcAgentCall> agents;
    std::vector<std::array<uint32_t, 3>> sent;  // type, data1, data2 of each MessageSend
    cIGZMessageTarget2* deliverTo = nullptr;    // the server's subscriber of the record
};

uint32_t HcSimDate(void* c) { return static_cast<HcWorld*>(c)->date; }
void HcNotification(void* c, cIGZMessageTarget2* t, uint32_t type, bool add) {
    static_cast<HcWorld*>(c)->notes.push_back({t, type, add});
}
void HcAgent(void* c, cIGZMessageTarget2* t, uint32_t level, const char* name, uint32_t flags, bool add) {
    static_cast<HcWorld*>(c)->agents.push_back({t, level, name ? name : "", flags, add});
}
void HcSend(void* c, cIGZMessage2* m) {
    HcWorld* w = static_cast<HcWorld*>(c);
    cIGZMessage2Standard* s = nullptr;
    if (m != nullptr && m->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&s)) && s != nullptr) {
        w->sent.push_back({m->GetType(), static_cast<uint32_t>(s->GetData1()), static_cast<uint32_t>(s->GetData2())});
        s->Release();
    }
    if (w->deliverTo != nullptr) w->deliverTo->DoMessage(m);  // as the message server would
}

FC4HistClockHooks HcHistoryHooks(HcWorld* w) {
    FC4HistClockHooks h{};
    h.struct_size = sizeof(h);
    h.ctx = w;
    h.sim_date = &HcSimDate;
    h.notification = &HcNotification;
    h.agent = &HcAgent;
    h.send = &HcSend;
    return h;
}

intptr_t HcInstall(FC4TestSeamFn seam, const FC4HistClockHooks* hooks) {
    FC4HistClockTestInput in{};
    in.op = 0;
    in.hooks = hooks;
    return seam(kFC4SeamHistClock, &in, nullptr);
}

uint32_t HcBits(float f) {
    uint32_t b = 0;
    std::memcpy(&b, &f, 4);
    return b;
}

// The §4.8 recording example through the record message, then the queries.
void HcHistoryRecordAndQuery(cFC4ISC4HistoryWarehouse* h, HcWorld& w) {
    const uint32_t K = 0x0A1100E8;
    const uint32_t d100 = kHcDay0 + 100;
    CHECK(h->SetHistoryDataQueueLength(K, 2));  // slot 13
    const struct {
        uint32_t day;
        int32_t value;
    } recs[] = {{d100, 5}, {d100 + 1, 6}, {d100 + 1, 7}, {d100 + 2, 8}};
    const size_t sentBefore = w.sent.size();
    for (const auto& r : recs) {
        w.date = r.day;
        CHECK(!h->RecordHistoryData(K, r.value));  // slot 12: MessageSend, delivered back; false (HW12)
    }
    CHECK(w.sent.size() == sentBefore + 4);
    if (w.sent.size() == sentBefore + 4) {
        CHECK(w.sent[sentBefore][0] == kHcMsgRecord && w.sent[sentBefore][1] == K && w.sent[sentBefore][2] == 5);
        CHECK(w.sent[sentBefore + 3][2] == 8);
    }
    // (101, 7), (102, 8): count, the exact ends, after the last point.
    float n = -1.0f;
    CHECK(h->GetCount(K, HcDate(d100), HcDate(d100 + 2), n) && n == 2.0f);  // slot 5
    float v = -1.0f;
    CHECK(h->GetPoint(K, HcDate(d100 + 2), v) && v == 8.0f);  // slot 8
    CHECK(h->GetPoint(K, HcDate(d100 + 50), v) && v == 8.0f);
    CHECK(h->GetPoint(K, HcDate(d100), v) && v == 7.0f);  // before the first point: the first value
    float mn = 0, mx = 0, avg = 0;
    CHECK(h->GetMinMaxAverage(K, HcDate(d100 + 1), HcDate(d100 + 2), mn, mx, avg));  // slot 7
    CHECK(mn == 7.0f && mx == 8.0f && avg == 7.5f);
    float slope = -9, icpt = -9;
    CHECK(h->GetTrend(K, HcDate(d100 + 1), HcDate(d100 + 2), slope, icpt));  // slot 6
    CHECK(slope == 1.0f && icpt == 7.0f);
    // A missing key: false, outputs untouched (§4.7, §9).
    n = -3.0f;
    mn = mx = avg = -4.0f;
    CHECK(!h->GetCount(0x12345678, HcDate(d100), HcDate(d100 + 2), n) && n == -3.0f);
    CHECK(!h->GetMinMaxAverage(0x12345678, HcDate(d100), HcDate(d100 + 2), mn, mx, avg));
    CHECK(mn == -4.0f && mx == -4.0f && avg == -4.0f);
    // The queue-length message (0x89EFA537) through the message target.
    if (w.deliverTo != nullptr) {
        HcMessage q(kHcMsgQueueLength, K, 1);
        CHECK(w.deliverTo->DoMessage(&q));
        CHECK(h->GetCount(K, HcDate(d100), HcDate(d100 + 2), n) && n == 1.0f);
        HcMessage rec(kHcMsgRecord, 0x2A11CB48, static_cast<intptr_t>(-17));
        w.date = d100 + 3;
        CHECK(w.deliverTo->DoMessage(&rec));
        CHECK(h->GetPoint(0x2A11CB48, HcDate(d100 + 3), v) && v == -17.0f);
    }
    // Slots 9-11: Coalesce and the two Disposes (the compiled order).
    const uint32_t K2 = 0x6A51178B;
    for (uint32_t i = 0; i < 4; ++i) {
        w.date = d100 + 10 + i;
        CHECK(!h->RecordHistoryData(K2, static_cast<int32_t>(i)));
    }
    CHECK(h->GetCount(K2, HcDate(d100), HcDate(d100 + 20), n) && n == 4.0f);
    CHECK(!h->Coalesce(0x12345678, HcDate(d100), HcDate(d100 + 20), 0));
    CHECK(h->Coalesce(K2, HcDate(d100), HcDate(d100 + 20), 0));  // one month: the last group stays open
    CHECK(h->DisposeRange(K2, HcDate(d100 + 10), HcDate(d100 + 13)));  // keeps the last point of the range
    CHECK(h->GetCount(K2, HcDate(d100), HcDate(d100 + 20), n) && n == 1.0f);
    CHECK(h->DisposeAll(K2));
    CHECK(!h->DisposeAll(0x12345678));
    // The yearly pass through the agent messages: SimNewYear arms it; each
    // SimNewDay step answers true or false (call again).
    if (w.deliverTo != nullptr) {
        HcMessage year(kHcMsgSimNewYear, 0);
        CHECK(w.deliverTo->DoMessage(&year));
        for (int i = 0; i < 6; ++i) {
            HcMessage day(kHcMsgSimNewDay, 0);
            w.deliverTo->DoMessage(&day);
        }
    }
}

// Queries from another thread while the main thread records (the lock).
void HcHistoryThreads(cFC4ISC4HistoryWarehouse* h, HcWorld& w) {
    std::atomic<bool> stop{false};
    std::atomic<int> answers{0};
    std::thread t([&] {
        float n = 0;
        while (!stop.load()) {
            if (h->GetCount(0x0A1100E8, HcDate(kHcDay0), HcDate(kHcDay0 + 100000), n)) ++answers;
        }
    });
    for (int i = 0; i < 2000; ++i) {
        w.date = kHcDay0 + 200 + static_cast<uint32_t>(i);
        h->RecordHistoryData(0x0A1100E8, i);
    }
    stop = true;
    t.join();
    CHECK(answers.load() > 0);
}

std::vector<uint8_t> HcWrite(cIGZSerializable* s) {
    LandfillOStream out;
    CHECK(s->Write(out));
    return out.bytes;
}

int HistoryEnableTest(const std::string& dll, const std::string& ini, const std::string& log) {
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    WriteTextFile(ini, "[subsystems]\nenable = history\n");
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (hd == nullptr) return 1;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    auto seam = reinterpret_cast<FC4TestSeamFn>(GetProcAddress(hd, "FC4TestSeam"));
    cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr && seam != nullptr);
    SetGamePrecision();
    if (director != nullptr && seam != nullptr) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        CHECK(classes.size() == 5);  // simulator (1) + budget (3) + history (1)
        CHECK(std::count(classes.begin(), classes.end(), kCLSID_History) == 1);

        static HcWorld w;
        const FC4HistClockHooks hooks = HcHistoryHooks(&w);
        CHECK(HcInstall(seam, &hooks) == 1);
        std::vector<uint8_t> record;
        void* obj = nullptr;
        CHECK(director->GetClassObject(kCLSID_History, kIID_cISC4HistoryWarehouse, &obj));
        auto* h = static_cast<cFC4ISC4HistoryWarehouse*>(obj);
        if (h != nullptr) {
            // §1: 0x69F4A803 and 1 at +0, cIGZSerializable at +8, nothing else -
            // not cIGZMessageTarget2 although the +4 sub-object exists [quirk].
            void* p = nullptr;
            CHECK(h->QueryInterface(GZIID_cIGZUnknown, &p) && p == static_cast<void*>(h));
            if (p != nullptr) static_cast<cIGZUnknown*>(p)->Release();
            p = nullptr;
            CHECK(h->QueryInterface(kIID_cISC4HistoryWarehouse, &p) && p == static_cast<void*>(h));
            if (p != nullptr) static_cast<cIGZUnknown*>(p)->Release();
            cIGZSerializable* ser = nullptr;
            CHECK(h->QueryInterface(GZIID_cIGZSerializable, reinterpret_cast<void**>(&ser)) && ser != nullptr);
            CHECK(reinterpret_cast<char*>(ser) == reinterpret_cast<char*>(h) + 8);
            if (ser != nullptr) CHECK(ser->GetGZCLSID() == kCLSID_History);
            p = reinterpret_cast<void*>(0x1234);
            CHECK(!h->QueryInterface(kHcIIDMessageTarget2, &p) && p == reinterpret_cast<void*>(0x1234));
            CHECK(!h->QueryInterface(0x12345678, &p) && p == reinterpret_cast<void*>(0x1234));
            CHECK(!h->QueryInterface(0x287FB697, &p) && p == reinterpret_cast<void*>(0x1234));
            // §3 Init: the +4 target subscribes to the record, then the queue
            // length; agents day (4) then year (7), flags 3.
            CHECK(h->Init());
            CHECK(w.notes.size() == 2 && w.agents.size() == 2);
            cIGZMessageTarget2* target = w.notes.empty() ? nullptr : w.notes[0].target;
            CHECK(reinterpret_cast<char*>(target) == reinterpret_cast<char*>(h) + 4);
            if (w.notes.size() == 2) {
                CHECK(w.notes[0].type == kHcMsgRecord && w.notes[0].add);
                CHECK(w.notes[1].type == kHcMsgQueueLength && w.notes[1].add && w.notes[1].target == target);
            }
            if (w.agents.size() == 2) {
                CHECK(w.agents[0].level == 4 && w.agents[0].flags == 3 && w.agents[0].add);
                CHECK(w.agents[1].level == 7 && w.agents[1].flags == 3 && w.agents[1].add);
                CHECK(w.agents[0].name == "cSC4HistoryWarehouse" && w.agents[1].name == "cSC4HistoryWarehouse");
                CHECK(w.agents[0].target == target);
            }
            w.deliverTo = target;
            HcHistoryRecordAndQuery(h, w);
            HcHistoryThreads(h, w);
            // §7: version 2, then the series in ascending key order.
            if (ser != nullptr) {
                record = HcWrite(ser);
                CHECK(record.size() > 6 && record[0] == 2 && record[1] == 0);
            }
            // §3 Shutdown: both unsubscribed, the agent removed; the series stay.
            CHECK(h->Shutdown());
            CHECK(w.notes.size() == 4 && w.agents.size() == 3);
            if (w.notes.size() == 4) CHECK(!w.notes[2].add && !w.notes[3].add);
            if (w.agents.size() == 3) CHECK(!w.agents[2].add && w.agents[2].target == target);
            float n = 0;
            CHECK(h->GetCount(0x0A1100E8, HcDate(kHcDay0), HcDate(kHcDay0 + 100000), n) && n == 1.0f);
            if (ser != nullptr) {
                CHECK(HcWrite(ser) == record);
                ser->Release();
            }
            w.deliverTo = nullptr;
            CHECK(h->Release() == 0);
        }
        // Read before Init (§7): the record loads into a new warehouse and
        // writes back byte for byte; version 3 is refused; a version-1
        // maxCount of 100 reads as 0.
        for (uint16_t version : {uint16_t{2}, uint16_t{3}}) {
            obj = nullptr;
            CHECK(director->GetClassObject(kCLSID_History, kIID_cISC4HistoryWarehouse, &obj));
            auto* fresh = static_cast<cFC4ISC4HistoryWarehouse*>(obj);
            if (fresh == nullptr) continue;
            cIGZSerializable* s = nullptr;
            CHECK(fresh->QueryInterface(GZIID_cIGZSerializable, reinterpret_cast<void**>(&s)) && s != nullptr);
            if (s != nullptr && !record.empty()) {
                std::vector<uint8_t> bytes = record;
                bytes[0] = static_cast<uint8_t>(version);
                LandfillIStream in(bytes);
                CHECK(s->Read(in) == (version == 2));
                if (version == 2) CHECK(HcWrite(s) == record);
            }
            if (s != nullptr) s->Release();
            CHECK(fresh->Release() == 0);  // never initialised: no unsubscription
        }
        {
            // v1: one series, key 7, maxCount 100, points (d, 1), (d, 2), (d + 1, 3).
            const uint32_t d = kHcDay0;
            std::vector<uint8_t> v1 = {1, 0};
            auto u32 = [&v1](uint32_t x) {
                for (int i = 0; i < 4; ++i) v1.push_back(static_cast<uint8_t>(x >> (8 * i)));
            };
            u32(1);
            u32(7);
            u32(100);
            u32(3);
            u32(d);
            u32(1);
            u32(d);
            u32(2);
            u32(d + 1);
            u32(3);
            obj = nullptr;
            CHECK(director->GetClassObject(kCLSID_History, kIID_cISC4HistoryWarehouse, &obj));
            auto* fresh = static_cast<cFC4ISC4HistoryWarehouse*>(obj);
            cIGZSerializable* s = nullptr;
            if (fresh != nullptr && fresh->QueryInterface(GZIID_cIGZSerializable, reinterpret_cast<void**>(&s))) {
                LandfillIStream in(v1);
                CHECK(s->Read(in));
                std::vector<uint8_t> want = {2, 0};
                auto put = [&want](uint32_t x) {
                    for (int i = 0; i < 4; ++i) want.push_back(static_cast<uint8_t>(x >> (8 * i)));
                };
                put(1);
                put(7);
                put(0);  // 100 -> 0 in a version-1 record
                put(2);
                put(d);
                put(2);  // the second 2000-01-01 entry overwrote the first
                put(d + 1);
                put(3);
                CHECK(HcWrite(s) == want);
                s->Release();
            }
            if (fresh != nullptr) CHECK(fresh->Release() == 0);
        }
        CHECK(HcInstall(seam, nullptr) == 1);
    }
    FreeLibrary(hd);
    const std::string text = ReadFile(log);
    const auto npos = std::string::npos;
    CHECK(text.find("subsystem 'history': enabled (1 class replaced) by FreeCity4.ini (enable=)") != npos);
    CHECK(text.find("history Init: ok=1") != npos);
    CHECK(text.find("off-main calls") != npos);
    CHECK(text.find("WARNING: released last reference") == npos && text.find("WARNING: over-release") == npos);
    if (g_failures == 0) std::printf("plugin history replacement test: all checks passed\n");
    return g_failures == 0 ? 0 : 1;
}

// ---- the history shadow (FC4HistoryShadow.h, seam op 54 op 1) --------------------------------------------

intptr_t HcShadow(FC4TestSeamFn seam, FC4HistoryShadowTestInput& in) {
    in.struct_size = sizeof(in);
    FC4HistClockTestInput t{};
    t.op = 1;
    t.arg = &in;
    return seam(kFC4SeamHistClock, &t, nullptr);
}

FC4HistoryShadowTestInput HcEvent(cIGZUnknown* orig, uint32_t event, uint32_t type = 0, uint32_t d1 = 0,
                                  uint32_t d2 = 0, int32_t julian = 0) {
    FC4HistoryShadowTestInput in{};
    in.event = event;
    in.original = orig;
    in.type = type;
    in.data1 = d1;
    in.data2 = d2;
    in.julian = julian;
    return in;
}

int HistoryShadowIniTest(const std::string& dll, const std::string& ini, const std::string& log) {
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    // With ours enabled the shadow is inactive in the game; the seam still
    // runs it, with FreeCity4's warehouse standing in for the original.
    WriteTextFile(ini, "[subsystems]\nenable = history\n[tools]\nhistory_shadow = 1\n");
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (hd == nullptr) return 1;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    auto seam = reinterpret_cast<FC4TestSeamFn>(GetProcAddress(hd, "FC4TestSeam"));
    cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr && seam != nullptr);
    SetGamePrecision();
    if (director != nullptr && seam != nullptr) {
        static HcWorld w;
        const FC4HistClockHooks hooks = HcHistoryHooks(&w);
        CHECK(HcInstall(seam, &hooks) == 1);
        void* obj = nullptr;
        CHECK(director->GetClassObject(kCLSID_History, kIID_cISC4HistoryWarehouse, &obj));
        auto* h = static_cast<cFC4ISC4HistoryWarehouse*>(obj);
        if (h != nullptr) {
            cIGZUnknown* orig = h;
            CHECK(h->Init());
            w.deliverTo = w.notes.empty() ? nullptr : w.notes[0].target;
            // Before the load: no shadow core, nothing taken.
            FC4HistoryShadowTestInput in = HcEvent(orig, 3);
            CHECK(HcShadow(seam, in) == -1);
            const uint32_t K = 0x0A1100E8, K2 = 0xAA11F837;
            CHECK(h->SetHistoryDataQueueLength(K, 3));
            for (uint32_t i = 0; i < 40; ++i) {
                w.date = kHcDay0 + 10 * i;
                h->RecordHistoryData(K, static_cast<int32_t>(i * i));
                h->RecordHistoryData(K2, static_cast<int32_t>(1000 - 7 * i));
            }
            // City load: the original's record read and written back, then
            // the queries on it.
            in = HcEvent(orig, 0, 0, 0, 0, static_cast<int32_t>(w.date));
            CHECK(HcShadow(seam, in) == 0);
            CHECK(in.compared > 2);
            // Messages reach both (as the message server delivers them).
            for (uint32_t i = 40; i < 80; ++i) {
                w.date = kHcDay0 + 10 * i;
                h->RecordHistoryData(K2, static_cast<int32_t>(3 * i));
                in = HcEvent(orig, 1, kHcMsgRecord, K2, 3 * i);
                CHECK(HcShadow(seam, in) == 1);
            }
            HcMessage q(kHcMsgQueueLength, 0x6A51178B, 2);
            w.deliverTo->DoMessage(&q);
            in = HcEvent(orig, 1, kHcMsgQueueLength, 0x6A51178B, 2);
            CHECK(HcShadow(seam, in) == 1);
            // The yearly pass through the agent deliveries: the shadow steps
            // its core after the original's step.
            w.date = kHcDay0 + 3650;  // 2009-12-29
            in = HcEvent(orig, 2, kHcMsgSimNewYear);
            CHECK(HcShadow(seam, in) == 1 && in.finished == 1);
            for (int i = 0; i < 4; ++i) {
                in = HcEvent(orig, 2, kHcMsgSimNewDay);
                CHECK(HcShadow(seam, in) == 1);
            }
            // The month comparison: exact.
            in = HcEvent(orig, 3, 0, 0, 0, static_cast<int32_t>(w.date));
            CHECK(HcShadow(seam, in) == 0);
            CHECK(in.compared > 100);
            // A record the shadow did not see: found, then the core is rebuilt.
            w.date += 1;
            h->RecordHistoryData(K2, 77);
            in = HcEvent(orig, 3, 0, 0, 0, static_cast<int32_t>(w.date));
            CHECK(HcShadow(seam, in) > 0);
            in = HcEvent(orig, 3, 0, 0, 0, static_cast<int32_t>(w.date));
            CHECK(HcShadow(seam, in) == 0);
            // City shutdown: the core dropped.
            in = HcEvent(orig, 4);
            CHECK(HcShadow(seam, in) == 0);
            in = HcEvent(orig, 3);
            CHECK(HcShadow(seam, in) == -1);
            CHECK(h->Shutdown());
            w.deliverTo = nullptr;
            CHECK(h->Release() == 0);
        }
        CHECK(HcInstall(seam, nullptr) == 1);
    }
    FreeLibrary(hd);
    const std::string text = ReadFile(log);
    const auto npos = std::string::npos;
    CHECK(text.find("history shadow: requested (history_shadow from FreeCity4.ini) but INACTIVE") != npos);
    CHECK(text.find("load: the original's record") != npos && text.find("written back identical; mismatches=0") != npos);
    CHECK(text.find("month: series=3 ") != npos);
    CHECK(text.find("record=") != npos && text.find("DIFFERENT") != npos);
    CHECK(text.find("history shadow (cont.) series 0xAA11F837") != npos);
    CHECK(text.find("rebuilt from the original's record") != npos);
    CHECK(text.find("years=1") != npos);
    CHECK(text.find("WARNING: released last reference") == npos && text.find("WARNING: over-release") == npos);
    if (g_failures == 0) std::printf("plugin history shadow test: all checks passed\n");
    return g_failures == 0 ? 0 : 1;
}

}  // namespace

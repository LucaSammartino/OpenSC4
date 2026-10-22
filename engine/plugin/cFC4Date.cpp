// SPDX-License-Identifier: GPL-3.0-or-later
// cIGZDate compiled ABI, docs/specs/dates.md §§3–4.
#include "cFC4Date.h"
#include "FC4Log.h"
#include "FC4GameList.h"
#include "cIGZString.h"
#include "fc4.h"
#include <stdexcept>

namespace {
std::atomic<unsigned> g_dateTraces{0};
thread_local bool g_dateFixture = false;
std::atomic<uint64_t> g_dateNatural{0}, g_dateFixtureCalls{0};
std::atomic<uint64_t> g_dateTimeRefused{0}, g_dateLocaleUnresolved{0};
}

cFC4Date::cFC4Date(int32_t julian, bool heapOwned)
    : julian_(static_cast<uint32_t>(julian)), heapOwned_(heapOwned) {}
uint32_t cFC4Date::Other(cFC4IGZDate const& d) {
    return const_cast<cFC4IGZDate&>(d).DayNumber();
}
uint32_t cFC4Date::Value(uint32_t slot, uint32_t a, uint32_t b, uint32_t c) const {
    (g_dateFixture ? g_dateFixtureCalls : g_dateNatural).fetch_add(1);
    const uint32_t n = julian_.load();
    const uint32_t result = fc4_gzdate_value(slot, n, a, b, c);
    if (fc4::LogLevel() >= fc4::kLogVerbose && g_dateTraces.fetch_add(1) < 200)
        FC4_LOGV("cFC4Date(%p)::slot%u day=%u -> %u", static_cast<const void*>(this), slot, n, result);
    return result;
}
uint32_t cFC4Date::Store(uint32_t n) { julian_.store(n); return n; }
bool cFC4Date::QueryInterface(uint32_t iid, void** out) {
    if (iid != 1 && iid != 0x43378255) return false;
    *out = static_cast<cFC4IGZDate*>(this);
    AddRef();
    return true;
}
uint32_t cFC4Date::AddRef() { return ++refCount_; }
uint32_t cFC4Date::Release() {
    uint32_t old = refCount_.load();
    while (old >= 2 && !refCount_.compare_exchange_weak(old, old - 1)) {}
    if (old >= 2) return old - 1;
    refCount_.store(0);
    if (heapOwned_) delete this;
    return 0;
}
uint32_t cFC4Date::SetMonthNameDayYear(const char* name, uint32_t d, uint32_t y) {
    g_dateLocaleUnresolved.fetch_add(1);
    // DT2: no fourth read, stop at NUL. Null faults before storing. DT4:
    // C-locale-only until an executable CRT map acquisition is published.
    uint8_t bytes[3]{};
    size_t len = 0;
    do { bytes[len] = static_cast<uint8_t>(name[len]); }
    while (bytes[len++] != 0 && len < 3);
    return SetMonthDayYear(fc4_gzdate_month(bytes, len, nullptr), d, y);
}
uint32_t cFC4Date::SetDayNumber(uint32_t n) { return Store(n); }
uint32_t cFC4Date::SetTime(cRZTime const&) {
    g_dateTimeRefused.fetch_add(1);
    // DT5: preserved unimplemented path; executable CRT timezone is missing.
    fc4::Log("dates: REFUSED SetTime (DT5); date unchanged");
    return 0;
}
uint32_t cFC4Date::SetMonthDayYear(uint32_t m, uint32_t d, uint32_t y) { return Store(Value(6, m, d, y)); }
uint32_t cFC4Date::SetDayOfYear(uint32_t d, uint32_t y) { return Store(Value(7, d, y)); }
uint32_t cFC4Date::Copy(cFC4IGZDate const& from) { return SetDayNumber(Other(from)); }
uint32_t cFC4Date::Clone(cFC4IGZDate** out) {
    auto* copy = new cFC4Date(static_cast<int32_t>(DayNumber()), true);
    *out = copy; // no guard or release of former pointee (DT3)
    return copy->AddRef();
}
bool cFC4Date::Between(cFC4IGZDate const& a, cFC4IGZDate const& b) { return Value(10, Other(a), Other(b)) != 0; }
int32_t cFC4Date::CompareTo(cFC4IGZDate const& d) { return static_cast<int32_t>(Value(11, Other(d))); }
uint32_t cFC4Date::WeekDay() { return Value(12); }
uint32_t cFC4Date::Year() { return Value(13); }
uint32_t cFC4Date::DayOfYear() { return Value(14); }
uint32_t cFC4Date::DayOfMonth() { return Value(15); }
uint32_t cFC4Date::FirstDayOfMonthOf(uint32_t m) { return Value(16, m); }
uint32_t cFC4Date::FirstDayOfMonth() { return Value(17); }
uint32_t cFC4Date::Hash() { return Value(18); }
bool cFC4Date::IsValid() { return Value(19) != 0; }
bool cFC4Date::Leap() { return Value(20) != 0; }
uint32_t cFC4Date::MaxDate(cFC4IGZDate const& d, cFC4IGZDate& out) {
    cFC4Date value(static_cast<int32_t>(Value(21, Other(d))));
    return out.Copy(value);
}
uint32_t cFC4Date::MinDate(cFC4IGZDate const& d, cFC4IGZDate& out) {
    cFC4Date value(static_cast<int32_t>(Value(22, Other(d))));
    return out.Copy(value);
}
uint32_t cFC4Date::Month() { return Value(23); }
uint32_t cFC4Date::Previous(uint32_t w, cFC4IGZDate& out) {
    cFC4Date value(static_cast<int32_t>(Value(24, w)));
    return out.Copy(value);
}
uint32_t cFC4Date::YearLastTwoDigits() { return Value(25); }
uint32_t cFC4Date::DayNumber() { return Value(26); }
bool cFC4Date::IsLess(cFC4IGZDate const& d) { return Value(27, Other(d)) != 0; }
bool cFC4Date::IsLessOrEqual(cFC4IGZDate const& d) { return Value(28, Other(d)) != 0; }
bool cFC4Date::IsGreater(cFC4IGZDate const& d) { return Value(29, Other(d)) != 0; }
bool cFC4Date::IsGreaterOrEqual(cFC4IGZDate const& d) { return Value(30, Other(d)) != 0; }
bool cFC4Date::IsEqual(cFC4IGZDate const& d) { return Value(31, Other(d)) != 0; }
bool cFC4Date::IsNotEqual(cFC4IGZDate const& d) { return Value(32, Other(d)) != 0; }
uint32_t cFC4Date::MinusDays(int32_t d) { return Value(33, static_cast<uint32_t>(d)); }
uint32_t cFC4Date::MinusDate(cFC4IGZDate const& d) { return Value(34, Other(d)); }
uint32_t cFC4Date::PlusDays(int32_t d) { return Value(35, static_cast<uint32_t>(d)); }
uint32_t cFC4Date::PlusDate(cFC4IGZDate const& d) { return Value(36, Other(d)); }
cFC4IGZDate& cFC4Date::Increment() { julian_.fetch_add(1); return *this; }
cFC4IGZDate& cFC4Date::Decrement() { julian_.fetch_sub(1); return *this; }
cFC4IGZDate& cFC4Date::AddDays(int32_t n) { julian_.fetch_add(static_cast<uint32_t>(n)); return *this; }
cFC4IGZDate& cFC4Date::SubtractDays(int32_t n) { julian_.fetch_sub(static_cast<uint32_t>(n)); return *this; }

uint32_t cFC4Date::DateString(cIGZString& out, uint32_t format) {
    char text[40]{};
    const size_t needed = fc4_gzdate_text(julian_.load(), format, reinterpret_cast<uint8_t*>(text), sizeof(text));
    fc4::compat::StlNodeFreeFn freeBuffer = nullptr;
    const void* vt = fc4::StlNodeAllocator::StringVtable();
    if (needed == 0 || needed > sizeof(text) || vt == nullptr ||
        !fc4::compat::StlNodeAllocator(nullptr, &freeBuffer)) {
        fc4::Log("dates: REFUSED DateString (native string allocator unavailable)");
        throw std::runtime_error("DateString requires native string allocator");
    }
    // DT3: actual native 8-byte initial buffer, native growth, owning Copy,
    // and native B cleanup. Allocation-null follows the published fault path.
    char* buffer = static_cast<char*>(fc4::StlNodeAllocator::Allocate(8));
    buffer[0] = 0;
    fc4::GameString temp{vt, buffer, buffer, buffer + 8, 0};
    auto& string = *reinterpret_cast<cIGZString*>(&temp);
    string.FromChar(text, static_cast<uint32_t>(needed - 1));
    out.Copy(string);
    buffer = temp.begin;
    uint32_t depth;
    // Public free typedef is void; §4.2 publishes EAX from this exact cleanup.
    __asm {
        push buffer
        call freeBuffer
        add esp, 4
        mov depth, eax
    }
    return depth;
}
bool cFC4Date::DayWithinMonth(uint32_t m, uint32_t d, uint32_t y) { return Value(42, m, d, y) != 0; }
uint32_t cFC4Date::DaysInYear(uint32_t y) { return Value(43, y); }
uint32_t cFC4Date::Jday(uint32_t m, uint32_t d, uint32_t y) { return Value(44, m, d, y); }
bool cFC4Date::LeapYear(uint32_t y) { return Value(45, y) != 0; }

#include "FC4DatesProbe.inc"

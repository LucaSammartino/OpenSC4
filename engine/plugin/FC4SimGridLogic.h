// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

// A layout-compatible cSC4SimGrid<T> (docs/specs/simgrid.md §2-§6).
//
// Original code reads and writes grid contents through the GetGridData
// pointer, assuming the game's STLport vector<vector<T>> (§6.3): an outer
// {begin, end, capacity_end} whose element x is row x, each row again
// {begin, end, capacity_end} over countZ contiguous T. This class keeps its
// data in exactly that shape, and every buffer comes from the allocator
// policy, in the game the STL node allocator B (FC4GameList.h
// StlNodeAllocator, docs/abi/allocators.md), which is what the original's
// default-allocator vectors use. So even game code that frees or reallocates
// a row hands the block back to the instance it came from.
//
// Only this class sizes the vectors (SetTractSize, Read; §6.4.5); outside
// code writes element values only.
//
// Header-only and free of game addresses: Alloc and Dims are policies, so
// tests run it with malloc and a fake city (engine/tests/simgrid_test.cpp).
//   Alloc: static void* Allocate(uint32_t), static void Free(void*)
//   Dims:  static bool Get(SimGridCity&) (false without a city), and
//          static void Trace(const char* what, uint32_t clsid, uint32_t instance,
//                            int32_t s, int32_t countX, int32_t countZ,
//                            uint32_t ifaceWrites, int changed, uint64_t digest,
//                            uint32_t nonzero) for Read/Write (digest: RecordDigest)
#pragma once

#include "cIGZIStream.h"
#include "cIGZOStream.h"
#include "cIGZSerializable.h"
#include "cISC4SimGrid.h"

#include <atomic>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <type_traits>

namespace fc4 {

constexpr uint32_t kSimGridIID = 0xC9BD939A;     // §2, all T
constexpr uint32_t kSimGridClsidBool = 0x49B9E601;  // creates a uint8 grid (§2)
constexpr uint16_t kSimGridVersion = 1;          // §5

// FNV-1a 64 (tooling: the record digest of the trace and the probe).
inline uint64_t SimGridFnv(uint64_t h, const void* p, size_t n) {
    const unsigned char* b = static_cast<const unsigned char*>(p);
    for (size_t i = 0; i < n; ++i) h = (h ^ b[i]) * 1099511628211ull;
    return h;
}
constexpr uint64_t kSimGridFnvBasis = 1469598103934665603ull;
// [choice] The largest tract count per axis Read accepts (a hostile save
// cannot make it allocate more); SC4's largest city is 256 cells.
constexpr int32_t kSimGridMaxCount = 4096;

// What SetTractSize reads from the city (§3).
struct SimGridCity {
    uint32_t cellCountX = 0;
    uint32_t cellCountZ = 0;
    float cellWidthX = 0.0f;
    float cellWidthZ = 0.0f;
};

// The game's vector layout (§6.3).
template <class E>
struct SimGridVec {
    E* begin;
    E* end;
    E* capacityEnd;
};
static_assert(sizeof(void*) != 4 || sizeof(SimGridVec<uint8_t>) == 12, "STLport vector: 12 bytes");

// Element type facts: CLSID (§2) and the stream calls of the record (§5).
template <class T> struct SimGridElem;
#define FC4_SIMGRID_ELEM(T, CLSID, GET, SET)                                    \
    template <> struct SimGridElem<T> {                                        \
        static constexpr uint32_t kClsid = CLSID;                              \
        static bool Get(cIGZIStream& s, T& v) { return s.GET(v); }             \
        static bool Set(cIGZOStream& s, T v) { return s.SET(v); }              \
    };
FC4_SIMGRID_ELEM(uint8_t, 0x49B9E602, GetUint8, SetUint8)
FC4_SIMGRID_ELEM(int8_t, 0x49B9E603, GetSint8, SetSint8)
FC4_SIMGRID_ELEM(uint16_t, 0x49B9E604, GetUint16, SetUint16)
FC4_SIMGRID_ELEM(int16_t, 0x49B9E605, GetSint16, SetSint16)
FC4_SIMGRID_ELEM(uint32_t, 0x49B9E606, GetUint32, SetUint32)
FC4_SIMGRID_ELEM(int32_t, 0x49B9E607, GetSint32, SetSint32)
FC4_SIMGRID_ELEM(uint64_t, 0x49B9E608, GetUint64, SetUint64)
FC4_SIMGRID_ELEM(int64_t, 0x49B9E609, GetSint64, SetSint64)
FC4_SIMGRID_ELEM(float, 0x49B9E60A, GetFloat32, SetFloat32)
FC4_SIMGRID_ELEM(double, 0x49B9E60B, GetFloat64, SetFloat64)
#undef FC4_SIMGRID_ELEM

// Process-wide counters for the trace (cFC4SimGrid.cpp logs them).
struct SimGridStats {
    std::atomic<uint32_t> created{0}, destroyed{0}, reads{0}, readFailures{0}, writes{0};
    std::atomic<uint32_t> resizes{0}, allocFailures{0}, gridDataCalls{0}, ifaceWrites{0};
    std::atomic<uint32_t> rawChanged{0};  // grids written with values changed since Read and no interface writes
};
inline SimGridStats& GetSimGridStats() {
    static SimGridStats s;
    return s;
}

// §3: the number of times S can be halved while it is > 1.
inline int32_t SimGridShift(int32_t s) {
    int32_t shift = 0;
    while (s > 1) {
        s /= 2;
        ++shift;
    }
    return shift;
}

// §4.1 step 3: the C runtime conversion of the quotient to an integer
// (_ftol: NaN and out-of-range give 0x8000000000000000), then truncated to T.
template <class T>
T SimGridQuotient(double q) {
    if constexpr (std::is_floating_point_v<T>) {
        return static_cast<T>(q);
    } else {
        int64_t i = INT64_MIN;
        if (q > -9223372036854775808.0 && q < 9223372036854775808.0) i = static_cast<int64_t>(q);
        return static_cast<T>(static_cast<uint64_t>(i));
    }
}

// §4.1 step 2: a value as a signed 64-bit integer (floats truncated).
template <class T>
int64_t SimGridAsInt64(T v) {
    if constexpr (std::is_floating_point_v<T>) {
        const double d = static_cast<double>(v);
        if (!(d > -9223372036854775808.0 && d < 9223372036854775808.0)) return INT64_MIN;  // _ftol
        return static_cast<int64_t>(d);
    } else {
        return static_cast<int64_t>(v);
    }
}

template <class T, class Alloc, class Dims>
class cFC4SimGridT final : public cISC4SimGrid<T>, public cIGZSerializable {
public:
    using Row = SimGridVec<T>;
    using Rows = SimGridVec<Row>;

    cFC4SimGridT() { GetSimGridStats().created.fetch_add(1, std::memory_order_relaxed); }

    // cIGZUnknown (slots 0-2)
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid == GZIID_cIGZUnknown || riid == kSimGridIID) {
            *ppvObj = static_cast<cISC4SimGrid<T>*>(this);
        } else if (riid == GZIID_cIGZSerializable) {
            *ppvObj = static_cast<cIGZSerializable*>(this);
        } else {
            return false;  // §4 slot 0: others false (*ppvObj untouched, as the original's switch)
        }
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return ++refCount_; }
    // §4 slot 2: count > 1: decrement and return it; else destroy (no Shutdown), return 0.
    uint32_t Release() override {
        if (refCount_ > 1) return --refCount_;
        delete this;
        return 0;
    }

    // slots 3-6
    bool Init() override {
        initFlag_ = true;
        return true;
    }
    bool Shutdown() override {
        initFlag_ = false;  // data kept
        return true;
    }
    uint32_t GetInstanceID() const override { return instance_; }
    bool SetInstanceID(uint32_t id) override {
        instance_ = id;
        return true;
    }

    // slots 7, 8
    T GetCellValue(int32_t cx, int32_t cz) const override { return At(cx >> shift_, cz >> shift_); }
    T GetAverageValueInCellRect(int32_t x, int32_t z, int32_t w, int32_t h) const override {
        if ((x >> shift_) == ((x + w - 1) >> shift_) && (z >> shift_) == ((z + h - 1) >> shift_)) {
            return At(x >> shift_, z >> shift_);
        }
        int64_t sum = 0;
        for (int32_t cx = x; cx < x + w; ++cx) {
            for (int32_t cz = z; cz < z + h; ++cz) sum += SimGridAsInt64(At(cx >> shift_, cz >> shift_));
        }
        return Average(sum, w, h);
    }

    // slots 9-21
    bool SetTractSize(int32_t s) override {
        if (s == tractSize_) return true;
        SimGridCity city;
        // [choice] Without a city the original would fault (it reads the
        // global city pointer); ours leaves the grid unchanged.
        if (!Dims::Get(city)) return true;
        const int32_t shift = SimGridShift(s);
        const int32_t countX = static_cast<int32_t>(city.cellCountX >> shift);
        const int32_t countZ = static_cast<int32_t>(city.cellCountZ >> shift);
        tractSize_ = s;
        shift_ = shift;
        widthX_ = city.cellWidthX * static_cast<float>(s);
        widthZ_ = city.cellWidthZ * static_cast<float>(s);
        invWidthX_ = 1.0f / widthX_;
        invWidthZ_ = 1.0f / widthZ_;
        Resize(countX, countZ);
        GetSimGridStats().resizes.fetch_add(1, std::memory_order_relaxed);
        return true;  // §4.2 [choice]
    }
    int32_t GetTractSize() const override { return tractSize_; }
    int32_t GetTractShift() const override { return shift_; }
    int32_t GetTractCountX() const override { return countX_; }
    int32_t GetTractCountZ() const override { return countZ_; }
    float GetTractWidthX() const override { return widthX_; }
    float GetTractWidthZ() const override { return widthZ_; }
    float GetOneOverTractWidthX() const override { return invWidthX_; }
    float GetOneOverTractWidthZ() const override { return invWidthZ_; }
    bool TractIsInBounds(uint32_t tx, uint32_t tz) const override {
        return tx < static_cast<uint32_t>(countX_) && tz < static_cast<uint32_t>(countZ_);
    }
    bool PositionToTract(float px, float pz, int32_t& tx, int32_t& tz) const override {
        tx = static_cast<int32_t>(px * invWidthX_);
        tz = static_cast<int32_t>(pz * invWidthZ_);
        return true;  // §4.3.1: non-zero AL
    }
    bool TractCornerToPosition(int32_t tx, int32_t tz, float& px, float& pz) const override {
        px = static_cast<float>(tx) * widthX_;
        pz = static_cast<float>(tz) * widthZ_;
        return true;
    }
    bool TractCenterToPosition(int32_t tx, int32_t tz, float& px, float& pz) const override {
        px = (static_cast<float>(tx) + 0.5f) * widthX_;
        pz = (static_cast<float>(tz) + 0.5f) * widthZ_;
        return true;
    }

    // slots 22-27
    T GetTractValue(int32_t tx, int32_t tz) const override { return At(tx, tz); }
    T GetAverageValueInTractRect(int32_t tx, int32_t tz, int32_t w, int32_t h) const override {
        int64_t sum = 0;
        for (int32_t x = tx; x < tx + w; ++x) {
            for (int32_t z = tz; z < tz + h; ++z) sum += SimGridAsInt64(At(x, z));
        }
        return Average(sum, w, h);
    }
    intptr_t GetGridData() override {
        GetSimGridStats().gridDataCalls.fetch_add(1, std::memory_order_relaxed);
        return reinterpret_cast<intptr_t>(&data_);
    }
    intptr_t GetGridData() const override {
        GetSimGridStats().gridDataCalls.fetch_add(1, std::memory_order_relaxed);
        return reinterpret_cast<intptr_t>(&data_);
    }
    void SetTractValue(int32_t tx, int32_t tz, T v) override {
        ++ifaceWrites_;
        GetSimGridStats().ifaceWrites.fetch_add(1, std::memory_order_relaxed);
        if (InBounds(tx, tz)) data_.begin[tx].begin[tz] = v;  // [choice] see At
    }
    void SetTractValues(T v) override {
        ++ifaceWrites_;
        GetSimGridStats().ifaceWrites.fetch_add(1, std::memory_order_relaxed);
        for (Row* r = data_.begin; r != data_.end; ++r) {
            for (T* e = r->begin; e != r->end; ++e) *e = v;
        }
    }

    // slot 28: the class destructor (§4.3.4)
    virtual ~cFC4SimGridT() {
        FreeAll();
        SimGridStats& st = GetSimGridStats();
        if (st.destroyed.fetch_add(1, std::memory_order_relaxed) + 1 == st.created.load(std::memory_order_relaxed)) {
            Dims::Trace("all-released", clsid_, instance_, tractSize_, 0, 0, 0, -1, 0, 0);
        }
    }

    // cIGZSerializable (§5)
    bool Write(cIGZOStream& s) override {
        s.SetUint16(kSimGridVersion);
        s.SetUint8(initFlag_ ? 1 : 0);
        s.SetUint32(clsid_);
        s.SetUint32(instance_);
        s.SetSint32(tractSize_);
        s.SetSint32(shift_);
        s.SetSint32(countX_);
        s.SetSint32(countZ_);
        s.SetFloat32(widthX_);
        s.SetFloat32(widthZ_);
        s.SetFloat32(invWidthX_);
        s.SetFloat32(invWidthZ_);
        for (int32_t x = 0; x < countX_; ++x) {
            for (int32_t z = 0; z < countZ_; ++z) SimGridElem<T>::Set(s, data_.begin[x].begin[z]);
        }
        GetSimGridStats().writes.fetch_add(1, std::memory_order_relaxed);
        // changed: -1 not loaded from a save, 0 same values as loaded, 1 changed
        const int changed = readSumValid_ ? (Checksum() != readSum_ ? 1 : 0) : -1;
        if (changed == 1 && ifaceWrites_ == 0) GetSimGridStats().rawChanged.fetch_add(1, std::memory_order_relaxed);
        uint32_t nonzero = 0;
        const uint64_t digest = RecordDigest(&nonzero);
        Dims::Trace("write", clsid_, instance_, tractSize_, countX_, countZ_, ifaceWrites_, changed, digest, nonzero);
        return true;  // §5: always true
    }
    bool Read(cIGZIStream& s) override {
        uint16_t version = 0;
        if (!s.GetUint16(version) || version != kSimGridVersion) return Fail();
        uint8_t init = 0;
        uint32_t clsid = 0, instance = 0;
        int32_t size = 0, shift = 0, countX = 0, countZ = 0;
        float wx = 0, wz = 0, ix = 0, iz = 0;
        const bool ok = s.GetUint8(init) && s.GetUint32(clsid) && s.GetUint32(instance) && s.GetSint32(size) &&
                        s.GetSint32(shift) && s.GetSint32(countX) && s.GetSint32(countZ) && s.GetFloat32(wx) &&
                        s.GetFloat32(wz) && s.GetFloat32(ix) && s.GetFloat32(iz);
        // §5 [choice]: false on a stream error; [choice] also on counts a
        // grid cannot hold (negative, or above kSimGridMaxCount).
        if (!ok || countX < 0 || countZ < 0 || countX > kSimGridMaxCount || countZ > kSimGridMaxCount) {
            return Fail();
        }
        // Every field as stored (the CLSID overwrites ours), nothing from the city.
        initFlag_ = init != 0;
        clsid_ = clsid;
        instance_ = instance;
        tractSize_ = size;
        shift_ = shift;
        widthX_ = wx;
        widthZ_ = wz;
        invWidthX_ = ix;
        invWidthZ_ = iz;
        if (!Resize(countX, countZ)) return Fail();
        for (int32_t x = 0; x < countX_; ++x) {
            for (int32_t z = 0; z < countZ_; ++z) {
                if (!SimGridElem<T>::Get(s, data_.begin[x].begin[z])) return Fail();
            }
        }
        ifaceWrites_ = 0;
        readSum_ = Checksum();
        readSumValid_ = true;
        GetSimGridStats().reads.fetch_add(1, std::memory_order_relaxed);
        uint32_t nonzero = 0;
        const uint64_t digest = RecordDigest(&nonzero);
        Dims::Trace("read", clsid_, instance_, tractSize_, countX_, countZ_, 0, -1, digest, nonzero);
        return true;
    }
    uint32_t GetGZCLSID() override { return clsid_; }

    // ---- ours (not reachable through the vtable) ----
    void SetClsid(uint32_t clsid) { clsid_ = clsid; }
    uint32_t RefCount() const { return refCount_; }
    bool InitFlag() const { return initFlag_; }
    uint32_t InterfaceWrites() const { return ifaceWrites_; }
    // FNV-1a over the values, x outer, z inner.
    // Tooling: FNV-1a 64 over the bytes Write puts into a little-endian
    // stream (the §5 record, header then x-major cells), so the digest of a
    // record captured from any grid's Write is comparable; `nonzero`: cells
    // whose value bytes are not all zero.
    uint64_t RecordDigest(uint32_t* nonzero) const {
        uint64_t h = kSimGridFnvBasis;
        const uint16_t version = kSimGridVersion;
        const uint8_t init = initFlag_ ? 1 : 0;
        h = SimGridFnv(h, &version, 2);
        h = SimGridFnv(h, &init, 1);
        h = SimGridFnv(h, &clsid_, 4);
        h = SimGridFnv(h, &instance_, 4);
        h = SimGridFnv(h, &tractSize_, 4);
        h = SimGridFnv(h, &shift_, 4);
        h = SimGridFnv(h, &countX_, 4);
        h = SimGridFnv(h, &countZ_, 4);
        h = SimGridFnv(h, &widthX_, 4);
        h = SimGridFnv(h, &widthZ_, 4);
        h = SimGridFnv(h, &invWidthX_, 4);
        h = SimGridFnv(h, &invWidthZ_, 4);
        uint32_t nz = 0;
        static const unsigned char kZero[sizeof(T)] = {};
        for (int32_t x = 0; x < countX_; ++x) {
            for (int32_t z = 0; z < countZ_; ++z) {
                const T& v = data_.begin[x].begin[z];
                h = SimGridFnv(h, &v, sizeof(T));
                if (std::memcmp(&v, kZero, sizeof(T)) != 0) ++nz;
            }
        }
        if (nonzero != nullptr) *nonzero = nz;
        return h;
    }

    uint64_t Checksum() const {
        uint64_t h = 1469598103934665603ull;
        for (const Row* r = data_.begin; r != data_.end; ++r) {
            const unsigned char* p = reinterpret_cast<const unsigned char*>(r->begin);
            const unsigned char* e = reinterpret_cast<const unsigned char*>(r->end);
            for (; p != e; ++p) h = (h ^ *p) * 1099511628211ull;
        }
        return h;
    }

private:
    bool InBounds(int32_t tx, int32_t tz) const {
        return tx >= 0 && tz >= 0 && tx < countX_ && tz < countZ_;
    }
    // §4: the original checks no bounds and reads outside the arrays.
    // [choice] Ours returns 0 there and ignores such writes (question SG1).
    T At(int32_t tx, int32_t tz) const { return InBounds(tx, tz) ? data_.begin[tx].begin[tz] : T{}; }

    T Average(int64_t sum, int32_t w, int32_t h) const {
        const int32_t n = static_cast<int32_t>(static_cast<uint32_t>(w) * static_cast<uint32_t>(h));
        // §4.1 [choice]: with w or h <= 0 (empty sum, 0 / 0) FreeCity4 returns 0.
        if (w <= 0 || h <= 0) return T{};
        return SimGridQuotient<T>(static_cast<double>(sum) / static_cast<double>(n));
    }

    bool Fail() {
        GetSimGridStats().readFailures.fetch_add(1, std::memory_order_relaxed);
        return false;
    }

    // §4.2: rows countX, each countZ; values kept where the indices still
    // exist, new elements 0. Every buffer from Alloc. On an allocation
    // failure the grid is left empty (counts 0) [choice]: original code then
    // sees zero rows, never a count larger than the data.
    bool Resize(int32_t countX, int32_t countZ) {
        Rows next{nullptr, nullptr, nullptr};
        bool ok = true;
        if (countX > 0) {
            next.begin = static_cast<Row*>(Alloc::Allocate(static_cast<uint32_t>(countX) * sizeof(Row)));
            ok = next.begin != nullptr;
            if (ok) {
                next.end = next.capacityEnd = next.begin + countX;
                for (Row* r = next.begin; r != next.end; ++r) *r = Row{nullptr, nullptr, nullptr};
                for (int32_t x = 0; ok && x < countX; ++x) {
                    Row& r = next.begin[x];
                    if (countZ == 0) continue;
                    r.begin = static_cast<T*>(Alloc::Allocate(static_cast<uint32_t>(countZ) * sizeof(T)));
                    if (r.begin == nullptr) {
                        ok = false;
                        break;
                    }
                    r.end = r.capacityEnd = r.begin + countZ;
                    std::memset(r.begin, 0, static_cast<size_t>(countZ) * sizeof(T));
                    if (x < countX_) {
                        const int32_t keep = countZ < countZ_ ? countZ : countZ_;
                        if (keep > 0) std::memcpy(r.begin, data_.begin[x].begin, static_cast<size_t>(keep) * sizeof(T));
                    }
                }
            }
        }
        if (!ok) {
            FreeRows(next);
            GetSimGridStats().allocFailures.fetch_add(1, std::memory_order_relaxed);
            FreeAll();
            return false;
        }
        FreeAll();
        data_ = next;
        countX_ = countX;
        countZ_ = countZ;
        return true;
    }
    static void FreeRows(Rows& rows) {
        for (Row* r = rows.begin; r != rows.end; ++r) {
            if (r->begin != nullptr) Alloc::Free(r->begin);
        }
        if (rows.begin != nullptr) Alloc::Free(rows.begin);
        rows = Rows{nullptr, nullptr, nullptr};
    }
    void FreeAll() {
        FreeRows(data_);
        countX_ = countZ_ = 0;
    }

    uint32_t refCount_ = 0;  // §6.1: a new grid has count 0
    bool initFlag_ = false;
    uint32_t clsid_ = SimGridElem<T>::kClsid;
    uint32_t instance_ = 0;
    int32_t tractSize_ = 0;  // §6.4.1: a fresh grid has S = 0 and no data
    int32_t shift_ = 0;
    int32_t countX_ = 0;
    int32_t countZ_ = 0;
    float widthX_ = 0.0f;
    float widthZ_ = 0.0f;
    float invWidthX_ = 0.0f;
    float invWidthZ_ = 0.0f;
    Rows data_{nullptr, nullptr, nullptr};
    // the trace (not saved)
    uint32_t ifaceWrites_ = 0;
    uint64_t readSum_ = 0;
    bool readSumValid_ = false;
};

}  // namespace fc4

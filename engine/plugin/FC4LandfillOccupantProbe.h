// SPDX-License-Identifier: GPL-3.0-or-later
// Internal to FC4LandfillShadow.cpp. Public-interface-only constructed probes
// for landfill.md sections 1, 2.3, 6.4, 7.3; no zone acquisition or calls.
#pragma once

#include "FC4WriteCapture.h"
#include "cIGZCOM.h"
#include "cISC4Occupant.h"
#include "cS3DVector3.h"

namespace fc4 {
namespace {

// The SDK's bool SetTileID is wrong; landfill.md 7.3 publishes EAX=old ID.
// Declare the corrected public interface, without addressing native fields.
class ProbeLandfillTile : public cIGZUnknown {
public:
    virtual bool Init() = 0;
    virtual bool Shutdown() = 0;
    virtual cISC4Occupant* AsOccupant() = 0;
    virtual uint32_t SetTileID(uint32_t) = 0;
    virtual int32_t GetOrientation() = 0;
    virtual bool SetOrientation(int32_t) = 0;
    virtual bool IsEmptyLandfillTile() = 0;
};

// Only primitives occur in these spec-authored, empty-holder records.
// Unsupported stream operations fail closed. This is never a native capture.
class OccupantProbeInput final : public cFC4ISC4DBSegmentIStream {
public:
    explicit OccupantProbeInput(const std::vector<uint8_t>& bytes) : bytes_(bytes) {}
    bool QueryInterface(uint32_t iid, void** out) override {
        if (!out) return false;
        *out = nullptr;
        if (iid != 1 && iid != kFC4_IID_cISC4DBSegmentIStream) return false;
        *out = static_cast<cFC4ISC4DBSegmentIStream*>(this);
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override { return --refs_; }
    bool Skip(uint32_t n) override { return Take(nullptr, n); }
    bool GetSint8(int8_t& v) override { return Take(&v, 1); }
    bool GetUint8(uint8_t& v) override { return Take(&v, 1); }
    bool GetSint16(int16_t& v) override { return Take(&v, 2); }
    bool GetUint16(uint16_t& v) override { return Take(&v, 2); }
    bool GetSint32(int32_t& v) override { return Take(&v, 4); }
    bool GetUint32(uint32_t& v) override { return Take(&v, 4); }
    bool GetSint64(int64_t& v) override { return Take(&v, 8); }
    bool GetUint64(uint64_t& v) override { return Take(&v, 8); }
    bool GetFloat32(float& v) override { return Take(&v, 4); }
    bool GetFloat64(double& v) override { return Take(&v, 8); }
    bool GetRZCharStr(char*, uint32_t) override { return Fail(); }
    bool GetGZStr(cIGZString&) override { return Fail(); }
    bool GetGZSerializable(cIGZSerializable& o) override { return o.Read(*this); }
    bool GetVoid(void* p, uint32_t n) override { return Take(p, n); }
    int32_t GetError() override { return good_ ? 0 : 1; }
    int32_t SetUserData(cIGZVariant*) override { return 0; }
    int32_t GetUserData() override { return 0; }
    bool Open(cISC4DBSegment*, cGZPersistResourceKey const&, bool) override { return Fail(); }
    bool Close() override { return true; }
    bool IsOpen() override { return true; }
    int32_t GetRecord() override { return 0; }
    int32_t GetSegment() override { return 0; }
    bool ReadGZSerializable(cIGZSerializable**, bool) override { return Fail(); }
    bool ReadResKey(cGZPersistResourceKey&) override { return Fail(); }
    bool ReadVariant(cIGZVariant&) override { return Fail(); }
    size_t Consumed() const { return pos_; }
private:
    bool Fail() { good_ = false; return false; }
    bool Take(void* out, size_t n) {
        if (!good_ || n > bytes_.size() - pos_) return Fail();
        if (out && n) memcpy(out, bytes_.data() + pos_, n);
        pos_ += n;
        return true;
    }
    const std::vector<uint8_t>& bytes_;
    size_t pos_ = 0;
    uint32_t refs_ = 1;
    bool good_ = true;
};

template <typename T>
void ProbePut(std::vector<uint8_t>& b, T v) {
    const auto* p = reinterpret_cast<const uint8_t*>(&v);
    b.insert(b.end(), p, p + sizeof(v));
}

std::vector<uint8_t> OccupantProbeRecord(uint8_t empty) {
    // landfill 6.4 + foundations 10.2/10.3: no properties or parent.
    std::vector<uint8_t> b;
    ProbePut<uint16_t>(b, 3);
    ProbePut<uint16_t>(b, 4);
    ProbePut<uint16_t>(b, 0x0102);
    ProbePut<uint8_t>(b, 0);
    ProbePut<uint8_t>(b, 0xF4);
    ProbePut<uint32_t>(b, 0x6A11BD2B);
    for (uint8_t c : {uint8_t(0xFF), uint8_t(0), uint8_t(0), uint8_t(0)}) ProbePut(b, c);
    ProbePut<uint16_t>(b, 2);  // exemplar holder
    ProbePut<uint16_t>(b, 2);  // plain holder
    ProbePut<uint32_t>(b, 0);  // own-property count
    for (unsigned i = 0; i < 3; ++i) ProbePut<uint32_t>(b, 0);  // parent key
    ProbePut<uint32_t>(b, 0x02F10010);
    ProbePut<int32_t>(b, -7);
    for (float v : {-24.5f, 12.25f, 40.0f}) ProbePut(b, v);
    ProbePut(b, empty);
    return b;
}

void OccupantProbeSink(void* user, const uint8_t* bytes, size_t len) {
    auto& b = *static_cast<std::vector<uint8_t>*>(user);
    b.insert(b.end(), bytes, bytes + len);
}

void RunLandfillOccupantProbe(void*) {
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    unsigned exact = 0, created = 0, freed = 0, reloads = 0;
    // Construction; valid read; mutation; byte==1; byte!=1; bad outer,
    // base and holder versions; truncation. Each case has its own lifetime.
    for (unsigned test = 0; test < 9; ++test) {
        auto input = test == 0 ? std::vector<uint8_t>{} : OccupantProbeRecord(test == 3 ? 1 : test == 4 ? 2 : 0);
        if (test == 5) input[0] = 2;
        if (test == 6) input[2] = 3;
        if (test == 7) input[16] = 1;
        if (test == 8) input.pop_back();
        const bool mutate = test == 2;
        const float xyz[3] = {88.5f, -4.0f, 120.25f};
        fc4_landfill_occupant_probe_result rust{};
        std::vector<uint8_t> expected;
        bool ok = fc4_landfill_occupant_probe(input.data(), input.size(), mutate ? 1u : 0u,
                                             -123, xyz, &OccupantProbeSink, &expected, &rust);
        void* p = nullptr;
        if (!com || !com->GetClassObject(0x8A22BEBE, 0x2A15F262, &p) || !p) {
            Log("landfill occupant probe case=%u INCOMPLETE factory", test);
            continue;
        }
        auto* tile = static_cast<ProbeLandfillTile*>(p);
        FC4_ACQUIRED(tile, "GetClassObject occupant probe");
        ++created;
        uint32_t qiMask = 0;
        unsigned bit = 0;
        for (uint32_t iid : {1u, 0x2A15F262u, 0x26BB767Eu, 0xE4FDA3D4u, 0xDEADBEEFu}) {
            void* q = nullptr;
            const bool answered = tile->QueryInterface(iid, &q);
            if (answered && q) {
                qiMask |= 1u << bit;
                auto* ref = static_cast<cIGZUnknown*>(q);
                FC4_ACQUIRED(ref, "QueryInterface occupant probe");
                if (iid == 0x26BB767E) ok = ok && q == tile->AsOccupant();
                if (iid == 0x2A15F262) ok = ok && q == tile;
                FC4_RELEASE(ref);
            } else if (answered) ok = false;
            ++bit;
        }
        const uint32_t added = FC4_ADDREF(tile);
        const uint32_t released = FC4_RELEASE(tile);
        ok = ok && added == released + rust.ref_delta && qiMask == rust.qi_mask;
        cIGZSerializable* serial = nullptr;
        if (tile->QueryInterface(0xE4FDA3D4, reinterpret_cast<void**>(&serial)) && serial) {
            FC4_ACQUIRED(serial, "QueryInterface occupant probe serializable");
            ok = ok && serial->GetGZCLSID() == rust.class_id;
            bool read = false;
            if (!input.empty()) {
                OccupantProbeInput stream(input);
                read = serial->Read(stream);
                if (read != (rust.read_ok != 0)) {
                    Log("landfill occupant probe case=%u Read native=%u rust=%u consumed=%u/%u error=%d", test,
                        read ? 1u : 0u, rust.read_ok, static_cast<unsigned>(stream.Consumed()),
                        static_cast<unsigned>(input.size()), stream.GetError());
                }
            }
            ok = ok && read == (rust.read_ok != 0);
            auto* occ = tile->AsOccupant();  // borrowed
            if (mutate && read && occ) {
                // [abi-ret] landfill.md 7.3: corrected interface returns the
                // previous uint32 ID in EAX, not the SDK's bool/AL.
                const uint32_t previous = tile->SetTileID(0x02F10010);
                ok = ok && previous == rust.previous_tile;
                const bool oriented = tile->SetOrientation(-123);
                ok = oriented && ok;
                const cS3DVector3 v(xyz[0], xyz[1], xyz[2]);
                const bool positioned = occ->SetPosition(&v);
                // LG32: landfill.md specifies the stored position/bounds,
                // not this inherited method's return. Compare its effects;
                // report the SDK bool separately without assuming true.
                Log("landfill occupant probe case=%u mutations previous native=0x%08X rust=0x%08X "
                    "SetOrientation=%u SetPosition=%u (return unscored LG32)", test, previous, rust.previous_tile,
                    oriented ? 1u : 0u, positioned ? 1u : 0u);
            }
            if (test == 0 || (read && rust.read_ok != 0)) {
                ok = ok && occ && tile->GetOrientation() == rust.orientation &&
                     tile->IsEmptyLandfillTile() == (rust.empty != 0);
                if (occ) {
                    cS3DVector3 pos, lo, hi;
                    ok = occ->GetPosition(&pos) && ok;
                    ok = ok && static_cast<uint32_t>(occ->GetType()) == rust.occupant_type &&
                         occ->GetFlags() == rust.flags && memcmp(&pos.fX, rust.position, 12) == 0;
                    if (read) {
                        occ->GetBoundingBox(&lo, &hi);
                        ok = ok && memcmp(&lo.fX, rust.bounds, 12) == 0 &&
                             memcmp(&hi.fX, rust.bounds + 3, 12) == 0;
                    }
                }
            }
            if (read && rust.read_ok != 0) {
                WriteCapture capture;
                const bool wrote = capture.Capture(tile);
                ok = ok && wrote == (rust.write_ok != 0) && capture.Bytes() == expected;
                if (capture.Bytes() != expected) {
                    size_t first = 0;
                    while (first < capture.Bytes().size() && first < expected.size() &&
                           capture.Bytes()[first] == expected[first]) ++first;
                    Log("landfill occupant probe case=%u record native=%u rust=%u first_diff=%u", test,
                        static_cast<unsigned>(capture.Bytes().size()), static_cast<unsigned>(expected.size()),
                        static_cast<unsigned>(first));
                }
                // Native persistence goes into a fresh factory object. Its
                // own captured record seeds only native Read, never Rust.
                void* fresh = nullptr;
                if (wrote && com->GetClassObject(0x8A22BEBE, 0xE4FDA3D4, &fresh) && fresh) {
                    auto* loaded = static_cast<cIGZSerializable*>(fresh);
                    FC4_ACQUIRED(loaded, "GetClassObject occupant probe reload");
                    ++created;
                    OccupantProbeInput reloadInput(capture.Bytes());
                    const bool loadedOk = loaded->Read(reloadInput);
                    void* tileRef = nullptr;
                    if (loadedOk && loaded->QueryInterface(0x2A15F262, &tileRef) && tileRef) {
                        auto* restored = static_cast<ProbeLandfillTile*>(tileRef);
                        FC4_ACQUIRED(restored, "QueryInterface occupant probe reload");
                        auto* restoredOcc = restored->AsOccupant();
                        cS3DVector3 restoredPos, lo, hi;
                        bool restoredOk = restoredOcc && restoredOcc->GetPosition(&restoredPos);
                        if (restoredOcc) {
                            restoredOcc->GetBoundingBox(&lo, &hi);
                            restoredOk = restoredOk && restoredOcc->GetFlags() == (rust.flags & 0xDD) &&
                                memcmp(&restoredPos.fX, rust.position, 12) == 0 &&
                                memcmp(&lo.fX, rust.bounds, 12) == 0 && memcmp(&hi.fX, rust.bounds + 3, 12) == 0;
                        }
                        WriteCapture second;
                        const bool secondOk = second.Capture(restored);
                        restoredOk = restoredOk && secondOk && second.Bytes() == expected &&
                            restored->GetOrientation() == rust.orientation &&
                            restored->IsEmptyLandfillTile() == (rust.empty != 0);
                        if (restoredOk) ++reloads;
                        ok = restoredOk && ok;
                        FC4_RELEASE(restored);
                    } else ok = false;
                    const uint32_t last = FC4_RELEASE_CREATED(loaded);
                    if (last == 0) ++freed;
                    ok = last == 0 && ok;
                } else ok = false;
            }
            FC4_RELEASE(serial);
        } else ok = false;
        // This reference came from our factory call; zero is direct evidence
        // of destruction, without dereferencing an object after its release.
        const uint32_t finalRef = FC4_RELEASE_CREATED(tile);
        if (finalRef == 0) ++freed;
        ok = ok && finalRef == 0;
        if (ok) ++exact;
        Log("landfill occupant probe case=%u %s qi=0x%X ref_add=%u ref_drop=%u final=%u read=%u write=%u",
            test, ok ? "exact" : "DIFFERENT", qiMask, added, released, finalRef, rust.read_ok, rust.write_ok);
    }
    Log("landfill occupant probe summary exact=%u/9 created=%u destroyed=%u reloads=%u/4; public factory/interfaces; "
        "authored inputs; SetPosition return unscored LG32; bad-holder Read difference LG33; no zone calls",
        exact, created, freed, reloads);
}

}  // namespace
}  // namespace fc4

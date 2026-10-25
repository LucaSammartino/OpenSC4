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

// cFC4LotConfiguration: the implementation of docs/specs/lot-configuration.md
// (see cFC4LotConfiguration.h). Every rule cites its section.
#include "cFC4LotConfiguration.h"

#include <windows.h>

#include "FC4Fpu.h"
#include "FC4Log.h"
#include "FC4GameContainers.h"
#include "FC4GameList.h"
#include "FC4Properties.h"
#include "FC4CoreProps.h"
#include "FC4RefAudit.h"
#include "cRZBaseString.h"

#include "cIGZPersistResourceManager.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4ZoneManager.h"
#include "cSC4LotConfigurationObject.h"

#include <algorithm>
#include <cmath>

namespace {

// 3.2: the exemplar properties.
constexpr uint32_t kPropName = 0x20;
constexpr uint32_t kPropFamily = 0x88EDC791;         // LotConfigPropertyFamily
constexpr uint32_t kPropWalls = 0x88EDC798;
constexpr uint32_t kPropFences = 0x88EDC797;

// 5.1 slot 4: the caller's string is assigned through its own operator=
// (string slot 13) and the reference to it is returned.
void AssignName(cIGZString& out, const std::vector<char>& name) {
    const std::string text(name.data(), name.size());
    out = cRZBaseString(text.c_str(), static_cast<uint32_t>(text.size()));
}

}  // namespace

cFC4LotConfiguration::cFC4LotConfiguration() {
    Reset();
}

cFC4LotConfiguration::~cFC4LotConfiguration() {
    fc4_lot_configuration_free(core_);
}

cIGZUnknown* CreateFC4LotConfiguration() {
    auto* config = new cFC4LotConfiguration();
    config->AddRef();
    return static_cast<cFC4ISC4LotConfiguration*>(config);
}

// 3.1 step 3: every field back to its default (2).
void cFC4LotConfiguration::Reset() {
    id_ = 0;
    name_.clear();
    width_ = 0;
    depth_ = 0;
    families_.clear();
    maxSlope_ = 90.0f;
    minSlope_ = 0.0f;
    maxSlopeFoundation_ = 90.0f;
    growthStage_ = 1;
    minCapacity_ = 0xFFFF;
    maxCapacity_ = 0;
    water_ = 0;
    zones_.assign(16, false);
    wealths_.assign(4, false);
    purposes_.assign(9, false);
    styles_.clear();
    monopoly_ = 0;
    doConstruction_ = 0;
    roadSides_ = 8;
    objects_.clear();
    walls_.clear();
    fences_.clear();
    fencePositions_.clear();
}

bool cFC4LotConfiguration::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    *ppvObj = nullptr;
    // 1: IIDs 1 and 0x68EDBEBC give the object (with a reference).
    if (riid == 1 || riid == kFC4_IID_cISC4LotConfiguration) {
        *ppvObj = static_cast<cFC4ISC4LotConfiguration*>(this);
        AddRef();
        return true;
    }
    return false;
}

uint32_t cFC4LotConfiguration::AddRef() {
    ++refCount_;
    return refCount_;
}

uint32_t cFC4LotConfiguration::Release() {
    // 1: a count above 1 is decremented and returned; otherwise the object
    // is deleted (its Shutdown is a no-op here - the configuration has no
    // Shutdown of its own, 0).
    if (refCount_ == 0 || --refCount_ != 0) {
        return refCount_;
    }
    // Diagnostic: a wrapper dying while the catalogue map still holds it
    // leaves a dangling configuration the game reads at the next growth pass
    // (AV reading 0 inside IsCompatibleWithPurposeType). Name the releaser.
    void* frames[16] = {};
    const USHORT n = RtlCaptureStackBackTrace(0, 16, frames, nullptr);
    char chain[400] = {};
    size_t at = 0;
    for (USHORT i = 1; i < n && at < sizeof(chain) - 16; ++i) {
        at += static_cast<size_t>(_snprintf_s(chain + at, sizeof(chain) - at, _TRUNCATE, "%s%p", i > 1 ? " " : "",
                                              frames[i]));
    }
    fc4::Log("lotcfg: configuration %p deleted (frames %u, releaser chain: %s)", static_cast<void*>(this),
             static_cast<unsigned>(n), chain[0] != 0 ? chain : "-");
    delete this;
    return 0;
}

uint32_t cFC4LotConfiguration::GetID() const { return id_; }  // 5.1

cIGZString& cFC4LotConfiguration::GetName(cIGZString& name) const {
    // 5.1 slot 4: the caller's string is assigned and the reference to it is
    // returned (a reference, not a bool: SDK error).
    AssignName(name, name_);
    return name;
}

bool cFC4LotConfiguration::BelongsToFamily(uint32_t family) const {
    // 5.2 slot 5: a linear scan of the family list.
    return std::find(families_.begin(), families_.end(), family) != families_.end();
}

SC4Vector<uint32_t>* cFC4LotConfiguration::GetFamilyIDArray() {
    // 5.1 slot 6: the address of the family list. The original returns its
    // own vector<u32>; ours is std::vector, whose layout is the same
    // (begin, end, capacity end).
    return reinterpret_cast<SC4Vector<uint32_t>*>(&families_);
}

void cFC4LotConfiguration::GetSize(uint32_t& w, uint32_t& d) const {
    // 5.1 slot 7: zero-extended; no defined result.
    w = width_;
    d = depth_;
}

float cFC4LotConfiguration::GetMaximumSlope() const { return maxSlope_; }                // 5.1
float cFC4LotConfiguration::GetMinimumSlope() const { return minSlope_; }                // 5.1
float cFC4LotConfiguration::GetMaximumSlopeBeforeFoundation() const {
    return maxSlopeFoundation_;
}  // 5.1

uint8_t cFC4LotConfiguration::GetGrowthStage() const { return growthStage_; }            // 5.1
uint16_t cFC4LotConfiguration::GetMinBuildingCapacity() const { return minCapacity_; }   // 5.1
uint16_t cFC4LotConfiguration::GetMaxBuildingCapacity() const { return maxCapacity_; }   // 5.1
bool cFC4LotConfiguration::IsCompatibleWithWater() const { return water_ != 0; }         // 5.1

bool cFC4LotConfiguration::IsCompatibleWithZoneType(uint32_t type) const {
    // 5.2: [choice] false outside the set (the original indexes without a
    // bound check).
    return type < zones_.size() && zones_[type];
}

bool cFC4LotConfiguration::IsCompatibleWithWealthType(uint32_t wealth) const {
    return wealth < wealths_.size() && wealths_[wealth];
}

bool cFC4LotConfiguration::IsCompatibleWithPurposeType(uint32_t purpose) const {
    return purpose < purposes_.size() && purposes_[purpose];
}

bool cFC4LotConfiguration::IsCompatibleWithStyleType(uint32_t group) const {
    // 5.2 slot 18: a linear scan of the style list (+0x80).
    return std::find(styles_.begin(), styles_.end(), group) != styles_.end();
}

// 5.4: always true; one ignored dword argument (pop 4).
bool cFC4LotConfiguration::IsCompatibleWithMonopolyPieceUse(uint32_t) {
    return true;
}

bool cFC4LotConfiguration::IsDoConstruction() const { return doConstruction_ != 0; }     // 5.1

void cFC4LotConfiguration::GetRequiredRoads(SC4Vector<bool>& roads) const {
    // 5.3 slot 21: `out` is resized to 4 and holds bit i of +0x8E. The
    // caller's object is the game's STLport vector<bool> (0x14 bytes, the
    // GameBitVector layout of FC4TractHost.cpp), not an SC4Vector<bool> - it
    // is written through that layout, as the readers do.
    struct BitVector {  // STLport _Bvector_base: {start, finish}, end_of_storage
        uint32_t* startWord;
        uint32_t startOffset;
        uint32_t* finishWord;
        uint32_t finishOffset;
        uint32_t* endOfStorage;
    };
    auto& v = *reinterpret_cast<BitVector*>(&roads);
    // [abi] The four bits go into one word's low bits: start = the word
    // buffer (4 bits, from allocator B), finish at offset 4.
    if (v.startWord != nullptr) {
        // Non-empty from a previous call: clear bit 0..3 in place.
        *v.startWord &= ~0xFu;
    } else {
        uint32_t* w = static_cast<uint32_t*>(fc4::StlNodeAllocator::Allocate(4));
        if (w == nullptr) return;
        *w = 0;
        v.startWord = w;
        v.startOffset = 0;
        v.finishWord = w;
        v.finishOffset = 0;
        v.endOfStorage = w + 1;
    }
    for (uint32_t i = 0; i < 4; ++i) {
        if ((roadSides_ >> i) & 1u) *v.startWord |= 1u << i;
    }
    v.finishWord = v.startWord;
    v.finishOffset = 4;
}

uint32_t cFC4LotConfiguration::GetLotConfigurationObjectCount() {
    // 5.1 slot 22: the number of records.
    return static_cast<uint32_t>(objects_.size());
}

SC4Vector<cSC4LotConfigurationObject>* cFC4LotConfiguration::GetLotConfigurationObjectArray() {
    // 5.1 slot 23: the address of the object array (borrowed).
    return reinterpret_cast<SC4Vector<cSC4LotConfigurationObject>*>(&objects_);
}

namespace {

// 5.1 slots 24-26: the first record whose type byte equals `type` (the
// enum's underlying value; the record's type byte is the game's v0).
cSC4LotConfigurationObject* FirstOf(std::vector<cSC4LotConfigurationObject>& v, uint8_t type) {
    for (cSC4LotConfigurationObject& o : v) {
        if (static_cast<uint8_t>(o.type) == type) return &o;
    }
    return nullptr;
}

}  // namespace

cSC4LotConfigurationObject* cFC4LotConfiguration::GetPrimaryBuildingConfigurationObject() {
    return FirstOf(objects_, 0);  // type 0: building
}

cSC4LotConfigurationObject* cFC4LotConfiguration::GetPrimaryBaseConfigurationObject() {
    return FirstOf(objects_, 2);  // type 2: base or overlay texture
}

cSC4LotConfigurationObject* cFC4LotConfiguration::GetPrimaryFoundationConfigurationObject() {
    return FirstOf(objects_, 3);  // type 3: foundation
}

bool cFC4LotConfiguration::HasObjectsOfType(uint32_t type) {
    // 5.2 slot 27: some record's type byte equals t (a t above 255 never
    // matches).
    if (type > 0xFFu) return false;
    return FirstOf(objects_, static_cast<uint8_t>(type)) != nullptr;
}

SC4Vector<uint32_t>* cFC4LotConfiguration::GetRetainingWallIDArray() {
    return reinterpret_cast<SC4Vector<uint32_t>*>(&walls_);   // 5.1 slot 28
}

SC4Vector<uint32_t>* cFC4LotConfiguration::GetFenceIDArray() {
    return reinterpret_cast<SC4Vector<uint32_t>*>(&fences_);  // 5.1 slot 29
}

SC4Vector<int8_t>* cFC4LotConfiguration::GetFenceSegmentPositionArray() {
    return reinterpret_cast<SC4Vector<int8_t>*>(&fencePositions_);  // 5.1 slot 30
}

bool cFC4LotConfiguration::Load(cGZPersistResourceKey const& key, uint8_t detail) {
    // 3.1: the whole load.
    FC4_X87_CHECK("lot configuration Load");
    fc4::props::Exemplar ex;
    // 3.1 step 1: the resource at key with the exemplar interface. None ->
    // return false with the object untouched.
    if (!ex.Load(key.type, key.group, key.instance)) return false;
    // 3.1 step 2: every property is read through the exemplar's property
    // holder, so inherited cohort properties count (the getters of
    // FC4Properties.h walk the cohorts).
    const cISCPropertyHolder* holder = ex.Holder();
    if (holder == nullptr) return false;

    // 3.1 step 3: reset, then step 4: name and ID.
    Reset();
    {
        std::vector<uint8_t> bytes;
        if (fc4::props::GetRawArrayBytes(holder, kPropName, bytes)) {
            name_.assign(bytes.begin(), bytes.end());
        }
        id_ = key.instance;
    }

    // Rust owns the parsed fields and geometry. Only the ABI adapter's arrays
    // are projected here; no native cSC4LotConfiguration is constructed.
    fc4::props::CoreProps properties(fc4::props::ToCoreAll(ex));
    fc4_lot_configuration_fields fields{};
    if (!fc4_lot_configuration_load(core_, properties.Get(), detail)) return false;
    if (!fc4_lot_configuration_snapshot(core_, &fields,
        [](void* ctx, const fc4_lotdev_cfg_object* v) {
            auto& objects = *static_cast<std::vector<cSC4LotConfigurationObject>*>(ctx);
            cSC4LotConfigurationObject o{};
            o.type = static_cast<cSC4LotConfigurationObject::Type>(v->object_type);
            o.flags = v->flags;
            o.rotation = static_cast<cSC4LotConfigurationObject::Rotation>(v->orientation);
            o.mirrored = v->byte3 != 0;
            o.x = v->offset[0]; o.y = v->offset[1]; o.z = v->offset[2];
            o.rectMaxX = v->footprint_max[0]; o.rectMaxZ = v->footprint_max[1];
            o.rectMinX = v->footprint_min[0]; o.rectMinZ = v->footprint_min[1];
            o.foundationIID = v->foundation;
            o.objectID = v->tag;
            for (size_t i = 0; i < v->n_ids; ++i) fc4::PushTrivial(o.data, v->ids[i]);
            objects.push_back(o);
        }, &objects_)) return false;
    width_ = static_cast<uint8_t>(fields.width);
    depth_ = static_cast<uint8_t>(fields.depth);
    maxSlope_ = fields.max_slope;
    minSlope_ = fields.min_slope;
    maxSlopeFoundation_ = fields.foundation_slope;
    growthStage_ = static_cast<uint8_t>(fields.growth_stage);
    water_ = static_cast<uint8_t>(fields.water);
    doConstruction_ = static_cast<uint8_t>(fields.construction);
    roadSides_ = static_cast<uint8_t>(fields.roads);
    for (size_t i = 0; i < zones_.size(); ++i) zones_[i] = ((fields.zones >> i) & 1u) != 0;
    for (size_t i = 0; i < wealths_.size(); ++i) wealths_[i] = ((fields.wealths >> i) & 1u) != 0;
    for (size_t i = 0; i < purposes_.size(); ++i) purposes_[i] = ((fields.purposes >> i) & 1u) != 0;

    // 3.2: the lot-level properties, in the table's order.
    {
        std::vector<uint32_t> u32s;
        families_.clear();
        if (fc4::props::GetU32s(holder, kPropFamily, u32s)) families_ = u32s;
        walls_.clear();
        if (fc4::props::GetU32s(holder, kPropWalls, u32s)) walls_ = u32s;
        // 3.4: fences (no shipped configuration has any; [choice] bounded
        // reads).
        fences_.clear();
        fencePositions_.clear();
        if (fc4::props::GetU32s(holder, kPropFences, u32s) && u32s.size() >= 2) {
            const uint32_t n = u32s[0];
            // 3.4 step 1: the types are v[2] .. v[n+1] (index 1 is skipped).
            const size_t typeCount = n <= u32s.size() - 2 ? n : u32s.size() - 2;
            for (size_t i = 0; i < typeCount; ++i) fences_.push_back(u32s[2 + i]);
            // 3.4 step 2: at index n+2, m segment records of four values
            // each (low bytes).
            const size_t at = static_cast<size_t>(n) + 2;
            if (at < u32s.size()) {
                const uint32_t m = u32s[at];
                const size_t total = m <= (u32s.size() - at - 1) / 4 ? m : (u32s.size() - at - 1) / 4;
                for (size_t i = 0; i < total; ++i) {
                    for (size_t j = 0; j < 4; ++j) {
                        fencePositions_.push_back(static_cast<int8_t>(u32s[at + 1 + i * 4 + j] & 0xFFu));
                    }
                }
            }
        }
    }

    // 3.1 steps 9-11: the capacity trim has no observable effect; Save is
    // never reached (version 2); the exemplar releases with `ex`.
    return true;
}

bool cFC4LotConfiguration::Save(cGZPersistResourceKey const&, void*) {
    // 7: editor only; the simulation never calls it and the class is never
    // saved (1). A stub is enough; false.
    return false;
}

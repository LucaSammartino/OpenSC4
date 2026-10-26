// SPDX-License-Identifier: GPL-3.0-or-later
// Included inside FC4AuraShadow's private namespace after Guarded.
// aura.md §14.3.1: retain the original helper's service results, then replay
// them through the independent cone implementation. No published map is input.
struct ConeProperty {
    uint32_t id;
    bool present = false;
    std::vector<int32_t> values;
};
struct ConeCapture {
    DWORD thread = 0;
    cISC4Occupant* occupant = nullptr;
    cISCPropertyHolder* holder = nullptr;
    bool replay = false, lotFound = false, departmentFound = false;
    uint32_t type = 0, department = 0;
    std::vector<cS3DVector3> positions;
    size_t position = 0;
    fc4_aura_lot lot{};
    ConeProperty properties[3]{{0x27812850}, {0x2781284F}, {0xCA5B9305}};
} g_cone;
NativeBoundary g_coneBoundary;
BoundarySlot g_coneType, g_conePosition, g_coneProperty, g_coneLot, g_coneDepartment;
fc4_aura_host g_coneLiveHost{};
bool InCone() { return g_cone.thread == GetCurrentThreadId(); }

int32_t __fastcall ConeType(cISC4Occupant* self, void*) {
    using Fn = int32_t(__thiscall*)(cISC4Occupant*);
    const int32_t result = reinterpret_cast<Fn>(g_coneType.original)(self);
    if (InCone() && self == g_cone.occupant) g_cone.type = static_cast<uint32_t>(result);
    return result;
}
bool __fastcall ConePosition(cISC4Occupant* self, void*, cS3DVector3* position) {
    using Fn = bool(__thiscall*)(cISC4Occupant*, cS3DVector3*);
    const bool result = reinterpret_cast<Fn>(g_conePosition.original)(self, position);
    if (InCone() && self == g_cone.occupant && position) g_cone.positions.push_back(*position);
    return result;
}
cISCProperty* __fastcall ConePropertyRead(const cISCPropertyHolder* self, void*, uint32_t id) {
    using Fn = cISCProperty*(__thiscall*)(const cISCPropertyHolder*, uint32_t);
    cISCProperty* result = reinterpret_cast<Fn>(g_coneProperty.original)(self, id);
    if (InCone() && self == g_cone.holder) {
        for (auto& property : g_cone.properties) {
            if (property.id != id) continue;
            property.present = result != nullptr;
            property.values.clear();
            if (result) {
                const auto* value = AsGame(result->GetPropertyValue());
                if (value && value->GetType() == (cIGZVariant::TypeArray | cIGZVariant::Sint32)) {
                    const auto count = value->GetCount();
                    const auto* data = value->RefSint32();
                    if (data && count) property.values.assign(data, data + count);
                }
                // §7's malformed-value choice remains in the core.
            }
        }
    }
    return result;
}
cISC4Lot* __fastcall ConeLot(cFC4ISC4LotManager* self, void*, int32_t x, int32_t z, bool flag) {
    using Fn = cISC4Lot*(__thiscall*)(cFC4ISC4LotManager*, int32_t, int32_t, bool);
    cISC4Lot* result = reinterpret_cast<Fn>(g_coneLot.original)(self, x, z, flag);
    if (InCone()) {
        g_cone.lotFound = result != nullptr;
        if (result) {
            SC4Rect<int32_t> rect;
            AsGame(result)->GetBoundingRect(rect);
            g_cone.lot.x1 = rect.topLeftX;
            g_cone.lot.z1 = rect.topLeftY;
            g_cone.lot.x2 = rect.bottomRightX;
            g_cone.lot.z2 = rect.bottomRightY;
        }
    }
    return result;
}
bool __fastcall ConeDepartment(cFC4ISC4BudgetSimulator* self, void*, cISCPropertyHolder* holder,
                                uint32_t purpose, cFC4ISC4BudgetSimulator::BudgetItem& item) {
    using Fn = bool(__thiscall*)(cFC4ISC4BudgetSimulator*, cISCPropertyHolder*, uint32_t,
                                 cFC4ISC4BudgetSimulator::BudgetItem&);
    const bool result = reinterpret_cast<Fn>(g_coneDepartment.original)(self, holder, purpose, item);
    if (InCone() && holder == g_cone.holder && purpose == 0xAA59670C) {
        g_cone.departmentFound = result;
        if (result) g_cone.department = item.departmentID;
    }
    return result;
}
bool ReplayHas(void* user, uint64_t occupant, uint32_t id) {
    if (!g_cone.replay) return g_coneLiveHost.occupant_has_property(user, occupant, id);
    // The original already applied the message gate; direct PostCityInit
    // helper calls have no preview-marker gate at all.
    if (id == 0x029244DB) return false;
    for (const auto& property : g_cone.properties) if (property.id == id) return property.present;
    return false;
}
bool ReplayProperty(void* user, uint64_t occupant, uint32_t id, fc4_props* out) {
    if (!g_cone.replay) return g_coneLiveHost.occupant_property(user, occupant, id, out);
    for (const auto& property : g_cone.properties) {
        if (property.id == id && property.present)
            return fc4_props_set_i32s(out, id, property.values.data(), property.values.size());
    }
    return false;
}
uint32_t ReplayType(void* user, uint64_t occupant) {
    return g_cone.replay ? g_cone.type : g_coneLiveHost.occupant_type(user, occupant);
}
void ReplayPosition(void* user, uint64_t occupant, float* out) {
    if (!g_cone.replay) { g_coneLiveHost.occupant_position(user, occupant, out); return; }
    if (g_cone.position >= g_cone.positions.size()) {
        Log("aura boundary: ERROR replay requested an uncaptured position");
        out[0] = out[1] = out[2] = 0;
        return;
    }
    const auto& position = g_cone.positions[g_cone.position++];
    out[0] = position.fX; out[1] = position.fY; out[2] = position.fZ;
}
bool ReplayLot(void* user, int32_t x, int32_t z, fc4_aura_lot* out) {
    if (!g_cone.replay) return g_coneLiveHost.lot_at(user, x, z, out);
    if (out && g_cone.lotFound) *out = g_cone.lot;
    return g_cone.lotFound;
}
bool ReplayDepartment(void* user, uint64_t occupant, uint32_t purpose, uint32_t* department) {
    if (!g_cone.replay) return g_coneLiveHost.budget_item_department(user, occupant, purpose, department);
    if (department && g_cone.departmentFound) *department = g_cone.department;
    return g_cone.departmentFound;
}
fc4_aura_host BoundaryHost(fc4_aura_host host) {
    g_coneLiveHost = host;
    host.occupant_has_property = &ReplayHas;
    host.occupant_property = &ReplayProperty;
    host.occupant_type = &ReplayType;
    host.occupant_position = &ReplayPosition;
    host.lot_at = &ReplayLot;
    host.budget_item_department = &ReplayDepartment;
    return host;
}
void __fastcall ConeDelivery(cISC4AuraSimulator* self, void*, cISC4Occupant* occupant, bool inserted) {
    using Fn = void(__thiscall*)(cISC4AuraSimulator*, cISC4Occupant*, bool);
    const auto original = reinterpret_cast<Fn>(g_coneBoundary.original);
    if (!Active() || !g_shadow || !g_followed || self != g_sim || !occupant || g_cone.thread) {
        original(self, occupant, inserted); return;
    }
    g_cone = ConeCapture{};
    g_cone.thread = GetCurrentThreadId();
    g_cone.occupant = occupant;
    g_cone.holder = occupant->AsPropertyHolder(); // borrowed; capture its original reads below
    auto* city = CurrentCity();
    const bool hooked = g_coneType.Install(occupant, 7, reinterpret_cast<void*>(&ConeType)) &&
        g_conePosition.Install(occupant, 8, reinterpret_cast<void*>(&ConePosition)) &&
        g_coneProperty.Install(g_cone.holder, 9, reinterpret_cast<void*>(&ConePropertyRead)) &&
        g_coneLot.Install(city->GetLotManager(), 6, reinterpret_cast<void*>(&ConeLot)) &&
        g_coneDepartment.Install(city->GetBudgetSimulator(), 51, reinterpret_cast<void*>(&ConeDepartment));
    original(self, occupant, inserted);
    g_coneType.Remove(); g_conePosition.Remove(); g_coneProperty.Remove();
    g_coneLot.Remove(); g_coneDepartment.Remove();
    if (hooked) {
        g_cone.replay = true;
        Call call;
        call.occupant = TokenOf(occupant);
        call.inserted = inserted;
        Guarded(city, "aura shadow (original cone inputs)", &CallOccupant, &call);
    } else {
        Log("aura boundary: ERROR original cone service capture incomplete");
    }
    g_cone.thread = 0;
    g_cone.replay = false;
}

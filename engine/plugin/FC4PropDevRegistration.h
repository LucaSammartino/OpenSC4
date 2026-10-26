// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "fc4.h"
#include <array>
#include <cmath>

namespace fc4 {
// PV18: insertion inputs belong to handler entry, before any state/timer
// setters or the post-service requester write. Never read the changed city
// to fill a missing replay input.
class PropRegistrationInput {
    uint64_t prop_ = 0;
    fc4_props* properties_ = nullptr;
    uint32_t state_ = 0, class_ = 0, type_ = 0, date_ = 0, trigger_ = 0;
    std::array<float, 3> position_{};
    int32_t x_ = 0, z_ = 0;
    bool powered_ = false, view_ = false, triggered_ = false;
    static bool Target(uint64_t p) {
        if (p == active->prop_) return true;
        active->valid = false;
        return false;
    }
    static fc4_props* Properties(void* u, uint64_t p) {
        if (!active) return base.prop_properties(u, p);
        return Target(p) ? fc4_props_clone(active->properties_) : nullptr;
    }
#define FC4_REG_VALUE(Name, Field) \
    static uint32_t Name(void* u, uint64_t p) { \
        if (!active) return base.Name(u, p); \
        return Target(p) ? active->Field : 0; \
    }
    FC4_REG_VALUE(prop_state, state_)
    FC4_REG_VALUE(prop_class, class_)
    FC4_REG_VALUE(prop_type, type_)
#undef FC4_REG_VALUE
    static void Position(void* u, uint64_t p, float* out) {
        if (!active) { base.prop_position(u, p, out); return; }
        if (Target(p) && out) for (size_t i = 0; i < 3; ++i) out[i] = active->position_[i];
    }
    static bool Powered(void* u, int32_t x, int32_t z) {
        if (!active) return base.is_powered(u, x, z);
        if (x == active->x_ && z == active->z_) return active->powered_;
        active->valid = false;
        return false;
    }
    static bool View(void* u) { return active ? active->view_ : base.view_available(u); }
    static bool Trigger(void* u, uint32_t t) {
        if (!active) return base.view_trigger_state(u, t);
        if (t == active->trigger_) return active->triggered_;
        active->valid = false;
        return false;
    }
    static uint32_t Date(void* u) { return active ? active->date_ : base.sim_date(u); }
public:
    inline static fc4_prop_host base{};
    inline static thread_local PropRegistrationInput* active = nullptr;
    bool valid = true;
    PropRegistrationInput() = default;
    PropRegistrationInput(const PropRegistrationInput&) = delete;
    PropRegistrationInput& operator=(const PropRegistrationInput&) = delete;
    ~PropRegistrationInput() { fc4_props_destroy(properties_); }
    class Scope {
        PropRegistrationInput* previous_;
    public:
        explicit Scope(PropRegistrationInput* input) : previous_(active) { active = input; }
        ~Scope() { active = previous_; }
        Scope(const Scope&) = delete;
        Scope& operator=(const Scope&) = delete;
    };
    static void Bind(fc4_prop_host& h) {
        base = h;
        h.prop_properties = &Properties;
        h.prop_state = &prop_state; h.prop_class = &prop_class; h.prop_type = &prop_type;
        h.prop_position = &Position; h.is_powered = &Powered;
        h.view_available = &View; h.view_trigger_state = &Trigger; h.sim_date = &Date;
    }
    void Capture(void* u, uint64_t prop, uint32_t trigger) {
        prop_ = prop; trigger_ = trigger;
        properties_ = base.prop_properties(u, prop);
        state_ = base.prop_state(u, prop);
        class_ = base.prop_class(u, prop);
        type_ = base.prop_type(u, prop);
        base.prop_position(u, prop, position_.data());
        x_ = static_cast<int32_t>(std::floor(position_[0] / 16.0f));
        z_ = static_cast<int32_t>(std::floor(position_[2] / 16.0f));
        powered_ = base.is_powered(u, x_, z_);
        view_ = base.view_available(u);
        triggered_ = trigger && view_ && base.view_trigger_state(u, trigger);
        date_ = base.sim_date(u);
    }
};
}

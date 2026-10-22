// SPDX-License-Identifier: GPL-3.0-or-later
// Clean-room drawing-fonts.md sections 1, 12 and 12.1; SDK virtual calls are MSVC thiscall.
#include "FC4FontSysVerify.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "compat/FC4Compat.h"
#include "cIGZCOM.h"
#include "cIGZFontSys.h"
#include "cRZCOMDllDirector.h"
#include <cstdint>

namespace fc4 {
void FontSysVerify(const char* phase) {
    if (!SettingInt("fontsys_verify", "FC4_FONTSYS_VERIFY", 0)) return;
    uint32_t exact = 0, diff = 0, refusal = 0;
    auto check = [&](bool value) { value ? ++exact : ++diff; };
    auto* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    void* service = nullptr;
    if (!compat::IdentifiedVersion() || !com ||
        !com->GetClassObject(0x20673960, 0x287FB697, &service) || !service) {
        Log("fontsys verify phase=%s constructed exact=0 diff=0 refusal=1 fallback=0 resync=0 natural=0 reason=factory", phase);
        return;
    }
    auto* ownedService = static_cast<cIGZUnknown*>(service);
    FC4_ACQUIRED(ownedService, "FontSys owned factory service view");
    void* primary = nullptr;
    if (ownedService->QueryInterface(0x00673960, &primary) && primary) {
        auto* owned = static_cast<cIGZFontSys*>(primary);
        FC4_ACQUIRED(owned, "FontSys owned primary QI");
        check(reinterpret_cast<uintptr_t>(service) == reinterpret_cast<uintptr_t>(primary) + 4);
        // Release only the created instance's views; never the framework's FontSys service.
        check(FC4_RELEASE_CREATED(ownedService) == 1);
        ownedService = nullptr;
        check(owned->Init());
        check(owned->Init());
        for (uint32_t iid : {uint32_t(1), uint32_t(0x00673960), uint32_t(0x287FB697), uint32_t(0), uint32_t(0xffffffff)}) {
            void* out = reinterpret_cast<void*>(uintptr_t(0x1234));
            const bool ok = owned->QueryInterface(iid, &out);
            check(ok == (iid == 1 || iid == 0x00673960 || iid == 0x287FB697));
            if (ok && out) {
                auto* view = static_cast<cIGZUnknown*>(out);
                FC4_ACQUIRED(view, "FontSys owned QI fixture");
                check(out == (iid == 0x287FB697 ? service : primary));
                check(FC4_RELEASE_CREATED(view) == 1);
            } else check(out == reinterpret_cast<void*>(uintptr_t(0x1234)));
        }
        check(owned->EnumerateFontInfo(nullptr, 0) == 0);
        check(owned->EnumerateManagedFonts(nullptr, 0) == 0);
        check(owned->Shutdown());
        check(owned->Shutdown());
        check(owned->Init());
        check(FC4_RELEASE_CREATED(owned) == 0);
    } else ++refusal;
    if (ownedService) FC4_RELEASE_CREATED(ownedService);
    Log("fontsys verify phase=%s constructed exact=%u diff=%u refusal=%u fallback=0 resync=0 natural=0 route=owned-factory-QI", phase, exact, diff, refusal);
}
}

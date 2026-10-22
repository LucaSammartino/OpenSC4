// SPDX-License-Identifier: GPL-3.0-or-later
// Constructed native-context conformance only: span-render-565 §§1.1-1.3, 6.1-6.2.
// The sibling fixtures supply architecture, never a pixel oracle or owner claim.
using Ctx565Void = void(__fastcall*)(void*, void*);
using Ctx565Init = uint8_t(__fastcall*)(void*, void*, void*);
using Ctx565Create = uint8_t(__fastcall*)(void*, void*, void*, void**);
using Ctx565Dword = void(__fastcall*)(void*, void*, uint32_t);
using Ctx565Color = void(__fastcall*)(void*, void*, uint32_t, uint32_t, const uint32_t*);
using Ctx565Gradient = void(__fastcall*)(void*, void*, const float*, float, float);
using Ctx565Source = uint8_t(__fastcall*)(void*, void*, void*, float, float, float, float, int32_t*);
using Ctx565Rect = void(__fastcall*)(void*, void*, const int32_t*);
using Ctx565Draw = void(__fastcall*)(void*, void*, const uint32_t*, uint32_t);

class Refusing565Source final : public cIGZUnknown {
public:
    uint32_t queries = 0, adds = 0, releases = 0;
    bool QueryInterface(uint32_t, void**) override { ++queries; return false; }
    uint32_t AddRef() override { ++adds; return 1; }
    uint32_t Release() override { ++releases; return 1; }
};

uint32_t Refs565(cIGZUnknown* p) {
    FC4_ADDREF(p);
    return FC4_RELEASE(p);
}
void Require565(bool pass) { if (!pass) ++g_contract565; }

void Draw565(void* ctx, const uint32_t* spans, uint32_t n, int slot, int site, bool allowNoChange = false) {
    g_expectedSlot565 = slot;
    g_expectedSite565 = site;
    g_allowNoChange565 = allowNoChange;
    for (size_t i = 0; i < g_dstBytes565; ++i) g_dst565[i] = uint8_t(i*37 + 0x5B);
    const uint64_t before = g_cases565;
    InterlockedExchange(&g_forceNative, 1);
    VSlot<Ctx565Draw>(ctx, 22)(ctx, nullptr, spans, n);
    InterlockedExchange(&g_forceNative, 0);
    if (g_cases565 != before + 1) ++g_unaccounted565;
}

void Trace() {
    g_traceThread = GetCurrentThreadId();
    cIGZBuffer* dst = MakeBuffer(17, 12, 5, 16);
    cIGZUnknown* ds = nullptr;
    void* ctx = nullptr;
    cIGZFrameWork* fw = RZGetFrameWork();
    bool initialized = false;
    if (dst && fw && fw->GetSystemService(0xEB2EA3CB, 0x4B2EA3B3, reinterpret_cast<void**>(&ds)) && ds) {
        FC4_ACQUIRED(ds, "565 fixture draw system");
        if (VEntry(ds, 4) == 0x0082BAB0) {
            const uint8_t made = VSlot<Ctx565Create>(ds, 4)(ds, nullptr, dst, &ctx);
            Require565(made == 1 && ctx);
            if (ctx) FC4_ACQUIRED(static_cast<cIGZUnknown*>(ctx), "565 fixture context");
        } else Require565(false);
        FC4_RELEASE(ds);
    }
    // Only vtables (public dispatch) are inspected, never native object fields.
    const uintptr_t entries[] = {0x00830710,0x0082FF20,0x0082FF70,0x0082FEA0,0x0082FEB0,
        0x0082FDB0,0x0082FFD0,0x0082FE60,0x0082FE70,0x008302A0,0x008302C0,0x008302E0};
    const unsigned slots[] = {3,4,7,8,9,12,13,14,15,17,18,22};
    bool boundary = ctx && *static_cast<uintptr_t*>(ctx) == 0x00AC17D0;
    if (boundary) for (unsigned i = 0; i < 12; ++i) boundary &= VEntry(ctx, slots[i]) == entries[i];
    if (boundary && !dst->IsLocked() && Refs565(dst) == 1 && Refs565(static_cast<cIGZUnknown*>(ctx)) == 1) {
        initialized = VSlot<Ctx565Init>(ctx, 3)(ctx, nullptr, dst) == 1;
    }
    if (initialized) {
        g_dst565 = static_cast<uint8_t*>(dst->GetColorSurfaceBits());
        g_dstBytes565 = dst->GetColorSurfaceStride() * 12;
        Require565(dst->IsLocked() && Refs565(dst) == 2);
        if (g_dst565 && dst->GetColorSurfaceStride() == 36 && dst->GetBytesPerPixel() == 2) {
            const uint32_t spans[] = {0x00030003,0x00030008,0x00040004,0x00040008,
                                      0x00030005,0x00030007,0x00050006,0x00050006};
            int32_t rect[4] = {111,222,333,444};
            Require565(VSlot<Ctx565Source>(ctx,13)(ctx,nullptr,nullptr,0,0,1,1,rect) == 0);
            VSlot<Ctx565Void>(ctx,14)(ctx,nullptr);
            VSlot<Ctx565Dword>(ctx,9)(ctx,nullptr,0);
            // Gradients are supplied as twelve binary32 floats to slot 12;
            // the original producer, not host field writes, builds fixed-point input.
            const float gradients[][12] = {
                {.25f,.5f,.75f,.8f, .0123f,-.015f,.031f,-.01f, .004f,.006f,-.003f,.005f},
                {.95f,.7f,.3f,.9f, -.013f,.011f,-.007f,-.009f, -.006f,.007f,.002f,-.004f},
                {0,0,0,0, 0,0,0,0, 0,0,0,0}};
            for (uint32_t argb : {0x80FF8040u,0xFF10C080u,0x07123456u}) {
                VSlot<Ctx565Color>(ctx,7)(ctx,nullptr,0,1,&argb);
                VSlot<Ctx565Gradient>(ctx,12)(ctx,nullptr,nullptr,0,0);
                // Signed alpha and low-byte modulation semantics, including -1.
                for (uint32_t a : {0u,2u,0xFFFFFFFFu,0x80000000u}) {
                    VSlot<Ctx565Dword>(ctx,8)(ctx,nullptr,a);
                    const bool alpha = a == 2;
                    Draw565(ctx,spans,4,alpha ? 3 : 1,alpha ? 2 : 0,alpha && (argb >> 24) < 8);
                }
            }
            for (const auto& gradient : gradients) for (unsigned alpha = 0; alpha < 2; ++alpha) {
                VSlot<Ctx565Gradient>(ctx,12)(ctx,nullptr,gradient,1.25f,-.5f);
                VSlot<Ctx565Dword>(ctx,8)(ctx,nullptr,alpha);
                Draw565(ctx,spans,4,alpha ? 4 : 2,alpha ? 3 : 1,alpha && &gradient == &gradients[2]);
            }
            VSlot<Ctx565Gradient>(ctx,12)(ctx,nullptr,nullptr,0,0);
            const uint32_t argb = 0x80C0A040;
            VSlot<Ctx565Color>(ctx,7)(ctx,nullptr,0,1,&argb);
            // Public source placement vectors; all sampled texels are in bounds.
            struct Placement { float x,y,sx,sy; uint32_t u,v,du,dv; int32_t rect[4]; int site; };
            const Placement placements[] = {
                {3,3,1,1,0,0,0x10000,0x10000,{3,3,12,9},4},
                {3,3,2,.5f,0xFFFEC000,0xFFFB0000,0x8000,0x20000,{3,3,21,6},5},
                {3,3,.5f,2,0xFFFB0000,0xFFFEC000,0x20000,0x8000,{3,3,7,15},5}
            };
            // A second narrow stretch vector is tested with one-pixel spans below.
            for (uint32_t type : {2u,4u,5u,6u,7u,8u,9u}) {
                cIGZBuffer* source = MakeBuffer(9,6,type,type == 2 ? 8 : type >= 8 ? 32 : 16);
                if (!source) { Require565(false); continue; }
                Require565(!source->IsLocked() && Refs565(source) == 1);
                if (!source->Lock(0x800)) { Require565(false); FC4_RELEASE_CREATED(source); continue; }
                g_src565 = static_cast<uint8_t*>(source->GetColorSurfaceBits());
                const uint32_t stride = source->GetColorSurfaceStride(), bpp = source->GetBytesPerPixel();
                g_srcBytes565 = stride*6;
                for (size_t i = 0; i < g_srcBytes565; ++i)
                    const_cast<uint8_t*>(g_src565)[i] = uint8_t(i*91 + type*13 + 0x17);
                Require565(source->Unlock(0x800) && !source->IsLocked());
                const uint32_t format = type == 7 ? 0 : type == 6 ? 1 : type == 4 ? 2 :
                                        type == 5 ? 3 : type == 9 ? 4 : type == 8 ? 5 : 6;
                const uint32_t masks[] = {0xFFF,0x7FFF,0x7FFF,0xFFFF,0xFFFFFF,0xFFFFFF,0};
                for (unsigned placement = 0; placement < 3; ++placement) {
                    const auto& p = placements[placement];
                    const unsigned modes = placement == 0 ? 2 : 1;  // plain plus tiled
                    Require565(VSlot<Ctx565Source>(ctx,13)(ctx,nullptr,source,p.x,p.y,p.sx,p.sy,rect) == 1);
                    Require565(std::memcmp(rect,p.rect,sizeof(rect)) == 0 && Refs565(source) == 2);
                    for (unsigned mode = 0; mode < modes; ++mode) {
                        const int32_t tile[] = {1,1,5,4};
                        const bool tiled = mode == 1;
                        uint32_t key = 0;
                        const size_t keyOffset = tiled ? stride + bpp : placement == 1 ? stride : placement == 2 ? bpp : 0;
                        std::memcpy(&key,g_src565+keyOffset,bpp);
                        if (tiled) VSlot<Ctx565Rect>(ctx,15)(ctx,nullptr,tile);
                        else VSlot<Ctx565Void>(ctx,14)(ctx,nullptr);
                        for (unsigned keyed = 0; keyed < 2; ++keyed) {
                            if (keyed) source->SetTransparency(key | (type >= 8 ? 0xFF000000 : 0));
                            else source->RemoveTransparency();
                            for (unsigned flags = 0; flags < 4; ++flags) {
                                VSlot<Ctx565Dword>(ctx,8)(ctx,nullptr,flags & 1);
                                VSlot<Ctx565Dword>(ctx,9)(ctx,nullptr,flags & 2 ? 2 : 0x100);
                                const int site = tiled ? 6 : p.site;
                                g_expectedKind565 = flags + (tiled ? 8 : p.site == 5 ? 4 : 0);
                                g_expectedSource565 = {g_src565,int32_t(stride),9,6,{type,bpp}};
                                g_expectedBlt565 = {p.u,p.v,p.du,p.dv,format,argb,key & masks[format],
                                    uint8_t(keyed && type != 2),{0,0,0}};
                                if (tiled) {
                                    g_expectedSource565.texels += stride+bpp;
                                    g_expectedSource565.width = 4; g_expectedSource565.height = 3;
                                    g_expectedBlt565.u0 = 1; g_expectedBlt565.v0 = 0;
                                } else if (p.site == 4) {
                                    g_expectedSource565.texels = reinterpret_cast<const uint8_t*>(
                                        reinterpret_cast<uintptr_t>(g_src565) - 3*bpp - 3*stride);
                                    g_expectedBlt565.u0 = 0; g_expectedBlt565.v0 = 0;
                                }
                                g_expectTexture565 = true;
                                const uint32_t small[] = {0x00030003,0x00030005};
                                Draw565(ctx,placement == 2 ? small : spans,placement == 2 ? 1 : 4,5,site);
                                Require565(!source->IsLocked() && Refs565(source) == 2 && dst->IsLocked());
                            }
                        }
                        Require565(tile[0] == 1 && tile[1] == 1 && tile[2] == 5 && tile[3] == 4);
                    }
                }
                // Null and failed-QI results are distinguished by sentinel output,
                // reference release, and the subsequent solid draw (no stale source).
                const int32_t sentinel[] = {111,222,333,444};
                std::memcpy(rect,sentinel,sizeof(rect));
                Require565(VSlot<Ctx565Source>(ctx,13)(ctx,nullptr,nullptr,0,0,1,1,rect) == 0);
                Refusing565Source bad;
                Require565(VSlot<Ctx565Source>(ctx,13)(ctx,nullptr,&bad,0,0,1,1,rect) == 1);
                Require565(std::memcmp(rect,sentinel,sizeof(rect)) == 0 && bad.queries == 1 &&
                    bad.adds == 0 && bad.releases == 0 && Refs565(source) == 1 && !source->IsLocked());
                g_expectTexture565 = false;
                VSlot<Ctx565Dword>(ctx,8)(ctx,nullptr,0);
                Draw565(ctx,spans,4,1,0);
                const uint32_t left = FC4_RELEASE_CREATED(source);
                Require565(left == 0);
                Log("span565 lifetime: source_type=%u public_lock_ref_restore=%s final_release=%u",
                    type, left == 0 ? "checked" : "DIFF",left);
            }
            VSlot<Ctx565Void>(ctx,17)(ctx,nullptr);  // Suspend, idempotence
            Require565(!dst->IsLocked());
            VSlot<Ctx565Void>(ctx,17)(ctx,nullptr);
            Require565(!dst->IsLocked());
            VSlot<Ctx565Void>(ctx,18)(ctx,nullptr);
            VSlot<Ctx565Void>(ctx,18)(ctx,nullptr);
            Require565(dst->IsLocked());
            VSlot<Ctx565Void>(ctx,17)(ctx,nullptr);
            Require565(!dst->IsLocked());
        } else Require565(false);
        VSlot<Ctx565Void>(ctx,4)(ctx,nullptr);  // Shutdown after Suspend: no leaked lock
        Require565(!dst->IsLocked() && Refs565(dst) == 1);
    } else Require565(false);
    if (ctx) Require565(FC4_RELEASE_CREATED(static_cast<cIGZUnknown*>(ctx)) == 0);
    if (dst) Require565(FC4_RELEASE_CREATED(dst) == 0);
    g_traceThread = 0;
    uint64_t rows = 0;
    for (unsigned row = 0; row < 14; ++row) for (unsigned kind = 0; kind < 12; ++kind)
        if (g_rows565[row][kind]) ++rows;
    Require565(rows == 156);  // row 13 is unreachable through the public context
    Log("span565 trace: constructed_native_context cases=%llu exact=%llu different=%llu refused=%llu fallback=0 resync=0 unaccounted=%llu compared_bytes=%llu changed_bytes=%llu controls=%llu unchanged_unexpected=%llu contract_failures=%llu rows=%llu/156 -> %s",
        g_cases565,g_exact565,g_diff565,g_refused565,g_unaccounted565,g_bytes565,g_changed565,g_noop565,
        g_unchanged565,g_contract565,rows,
        g_cases565 && g_cases565 == g_exact565 && !g_diff565 && !g_refused565 && !g_unaccounted565 &&
        !g_unchanged565 && !g_contract565 ? "EXACT" : "FAIL");
    Log("span565 trace: native_sites solid=%ld gradient=%ld alpha=%ld alpha_gradient=%ld plain=%ld stretch=%ld tiled=%ld; constructed fixtures do not establish SR565-1/5 shipped use",
        g_sites565[0],g_sites565[1],g_sites565[2],g_sites565[3],g_sites565[4],g_sites565[5],g_sites565[6]);
}

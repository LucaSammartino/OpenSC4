// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
#include <limits>

namespace fc4 { namespace prop_verify {
// Frozen literal oracle, prop-occupant sections 2.2/4.1 and network-tools 13.1.
// No native observations enter these values. Timer fixtures never reach low F.
struct Scalars { int32_t orientation; uint32_t state, chance, usage, timer; };
constexpr Scalars defaults{0, 0, 100, 0, 0};
struct Fixture { unsigned object, slot; int32_t arg; bool on; Scalars expected; };
constexpr Fixture fixtures[] = {
    {0,10,(std::numeric_limits<int32_t>::min)(),false,{0,0,100,0,0}},
    {0,10,-1,false,{0,0,100,0,0}},
    {0,10,0,false,{0,0,100,0,0}},
    {0,10,1,false,{1,0,100,0,0}},
    {0,10,2,false,{2,0,100,0,0}},
    {0,10,3,false,{3,0,100,0,0}},
    {0,10,4,false,{3,0,100,0,0}},
    {0,10,(std::numeric_limits<int32_t>::max)(),false,{3,0,100,0,0}},
    {0,10,-128,false,{0,0,100,0,0}},
    {0,17,0,false,{0,0,0,0,0}},
    {0,17,1,false,{0,0,1,0,0}},
    {0,17,99,false,{0,0,99,0,0}},
    {0,17,100,false,{0,0,100,0,0}},
    {0,17,255,false,{0,0,255,0,0}},
    {0,23,0,false,{0,0,255,0,0}},
    {0,23,1,false,{0,0,255,1,0}},
    {0,23,2,false,{0,0,255,2,0}},
    {0,23,3,false,{0,0,255,3,0}},
    {0,23,4,false,{0,0,255,4,0}},
    {0,23,255,false,{0,0,255,255,0}},
    {0,11,0,false,{0,0,255,255,0}},
    {0,11,1,false,{0,1,255,255,0}},
    {0,11,1,false,{0,1,255,255,0}},
    {0,11,2,false,{0,2,255,255,0}},
    {0,11,255,false,{0,255,255,255,0}},
    {0,11,128,false,{0,128,255,255,0}},
    {0,11,0,false,{0,0,255,255,0}},
    {0,21,0x80,true,{0,0,255,255,0x80}},
    {0,21,0x40,true,{0,0,255,255,0xc0}},
    {0,21,0x03,true,{0,0,255,255,0xc3}},
    {0,21,0x04,true,{0,0,255,255,0xc7}},
    {0,21,0x02,false,{0,0,255,255,0xc5}},
    {0,21,0x08,true,{0,0,255,255,0xcd}},
    {0,21,0x80,false,{0,0,255,255,0x4d}},
    {0,21,0x40,false,{0,0,255,255,0x0d}},
    {0,21,0x01,false,{0,0,255,255,0x0c}},
    {0,21,0x0c,false,{0,0,255,255,0}},
    {0,21,0xf0,true,{0,0,255,255,0xf0}},
    {0,21,0xf0,false,{0,0,255,255,0}},
    {0,21,0,true,{0,0,255,255,0}},
    {1,10,2,false,{2,0,100,0,0}},
    {1,17,1,false,{2,0,1,0,0}},
    {1,11,255,false,{2,255,1,0,0}},
    {1,23,3,false,{2,255,1,3,0}},
    {1,21,0x80,true,{2,255,1,3,0x80}},
};
constexpr uint8_t timerMasks[] = {0, 1, 2, 8, 0x80, 0xff};
} }

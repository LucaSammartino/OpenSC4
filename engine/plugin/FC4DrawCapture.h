// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
class cISC4City;
namespace fc4 {
// Default off. Hooks are installed in memory after executable identification.
// Frames are framework OnTick intervals, as in simulator.md 5.8.
void DrawCaptureStartup();
void DrawCaptureTick(cISC4City* city);
}

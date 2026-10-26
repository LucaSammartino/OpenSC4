// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include "cIGZUnknown.h"
#include "SC4Percentage.h"
#include <cstdint>

class cIGZString;
class cIGZBuffer;
class cISC4BuildingOccupant;
class cGZPersistResourceKey;
class cISCResExemplar;
class SC4String;

namespace fc4 {
// mysims.md §7.2: explicit compiled order, including the grouped job-title
// overload and the grade methods' hidden result pointer + group argument.
class MySimPublic : public cIGZUnknown {
public:
    virtual void* AsAdvisor() = 0; // 3
    virtual void MoveIn(uint8_t, cIGZString&, bool, uint8_t, SC4String&, uint32_t, SC4String&) = 0;
    virtual bool FindNewResidence() = 0;
    virtual bool ChangeResidence(cISC4BuildingOccupant*) = 0;
    virtual bool TestForResidence(cISC4BuildingOccupant*, bool, bool) = 0;
    virtual bool ChangeJob(cISC4BuildingOccupant*) = 0;
    virtual bool DeleteFromGame() = 0;
    virtual uint8_t GetSlot() = 0;
    virtual cISC4BuildingOccupant* GetResidence() = 0;
    virtual cISC4BuildingOccupant* GetJob() = 0;
    virtual bool SetResidence(cISC4BuildingOccupant*) = 0;
    virtual bool GetLocation(float*) = 0;
    virtual void GetVehicleKey(cGZPersistResourceKey&) = 0;
    virtual uint8_t GetSign() = 0;
    virtual uint8_t GetAge() = 0;
    virtual bool IsMale() = 0;
    virtual uint32_t GetWealth() = 0;
    virtual bool IsActive() = 0; // 20
    virtual SC4String* GetBio() = 0;
    virtual uint32_t GetBitmapID() = 0;
    virtual SC4String* GetBitmapFile() = 0;
    virtual bool GetBitmapBuffer(cIGZBuffer**) = 0;
    virtual SC4String* GetHomeName() = 0;
    virtual SC4String* GetJobName() = 0;
    virtual void GetBuildingJobTitle(cISC4BuildingOccupant*, cIGZString&) = 0; // 27
    virtual SC4String* GetJobTitle() = 0;
    virtual bool GetEQAsText(cIGZString&) = 0;
    virtual bool GetHQAsText(cIGZString&) = 0;
    virtual void Find() = 0;
    virtual void ShowResidence() = 0;
    virtual void HideResidence() = 0;
    virtual void ShowLocation() = 0;
    virtual void HideLocation() = 0;
    virtual void SetPylonVisibility(bool) = 0;
    virtual void UpdateResidence() = 0;
    virtual void LeaveForWork() = 0;
    virtual void ReturnHome() = 0;
    virtual void AnnounceMoveIn() = 0; // 40
    virtual void DoScriptCommand(uint32_t) = 0;
    virtual void DoBuildingAdded(cISC4BuildingOccupant*) = 0;
    virtual void DoBuildingChanged(cISC4BuildingOccupant*) = 0;
    virtual void DoBuildingRemoved(cISC4BuildingOccupant*, uint32_t) = 0;
    virtual void DoFireAdded(float, float) = 0;
    virtual void DoPipeBurst(float, float) = 0;
    virtual void DoTornado(float, float) = 0;
    virtual void DoDisaster(uint32_t, int32_t, int32_t) = 0;
    virtual void DoNeighborConnectionRemoved(int32_t, int32_t) = 0;
    virtual uint32_t DistanceToClosest(uint32_t) = 0;
    virtual uint32_t DistanceToClosestSpecial(uint32_t) = 0;
    virtual SC4Percentage* LocalSchoolGrade(SC4Percentage*, uint32_t) = 0;
    virtual SC4Percentage* LocalHospitalGrade(SC4Percentage*, uint32_t) = 0;
    virtual void PlayNightSound() = 0;
    virtual bool GetCurrentScreenLocationXY(int32_t*, int32_t*) = 0;
    virtual bool RequestToPlaySound() = 0;
    virtual bool SetCar(const cISCResExemplar*) = 0;
    virtual bool SetPedestrian(const cISCResExemplar*) = 0;
    virtual uint32_t GetModelButtonBitmapID(bool) = 0;
    virtual void GetNextBalloonAdvice() = 0; // 60
    virtual void GetNextBalloonEventAdvice() = 0;
    virtual uint32_t GetCurrentBalloonAdvice() = 0;
    virtual uint32_t GetCurrentBalloonEventAdvice() = 0;
    virtual bool CanUDriveIt() = 0;
    virtual bool EnterUDriveIt() = 0;
    virtual bool ExitUDriveIt() = 0;
    virtual bool CreateDispatchTool() = 0;
    virtual bool ActivateLuaContext() = 0; // 68
};
} // namespace fc4

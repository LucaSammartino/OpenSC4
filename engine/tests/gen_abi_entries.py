# SPDX-License-Identifier: GPL-3.0-or-later
#
# FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
# Copyright (C) 2026 FreeCity4 contributors
#
# This program is free software: you can redistribute it and/or modify it
# under the terms of the GNU General Public License as published by the Free
# Software Foundation, either version 3 of the License, or (at your option)
# any later version. This program is distributed WITHOUT ANY WARRANTY; see
# the GNU General Public License for more details. You should have received a
# copy of the license along with this program (see LICENSE at the repository
# root); if not, see <https://www.gnu.org/licenses/>.

"""Writes tests/abi_entries.inc: for every interface in docs/abi/interfaces.tsv
that the plugin declares, the list of its virtual methods as member-function
pointers. The test takes everything else from the compiler: the slot (from the
vcall thunk), the argument bytes and the return channel (from the type).

Only the method NAMES come from this script. A non-overloaded method is
written as &Class::Name, so a changed signature is picked up without
regenerating; an overloaded SDK method needs its type spelled out (a changed
signature then fails to compile, which is the cue to regenerate). A method
added to a declaration but missing here shows up as a gap in the slot
numbers, which the test reports.

Usage (from engine/): python tests/gen_abi_entries.py
"""

import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ENGINE = os.path.dirname(HERE)
SDK = os.path.join(ENGINE, "third_party", "gzcom-dll", "gzcom-dll", "include")
FC4 = os.path.join(ENGINE, "plugin", "FC4Interfaces.h")
FC4C = os.path.join(ENGINE, "plugin", "FC4CrimeInterfaces.h")
FCD = os.path.join(ENGINE, "plugin", "FC4DevelopmentInterfaces.h")
FC4T = os.path.join(ENGINE, "plugin", "FC4TractInterfaces.h")
FCL = os.path.join(ENGINE, "plugin", "FC4LotDevInterfaces.h")
FCW = os.path.join(ENGINE, "plugin", "FC4WinInterfaces.h")
FCP = os.path.join(ENGINE, "plugin", "FC4PortInterfaces.h")

# (table interface, class, header, required)
#   required = the plugin implements or calls the interface through this
#   declaration: a mismatch fails the test. Otherwise it is reported only.
INTERFACES = [
    ("cISC4Simulator", "cFC4ISC4Simulator", FC4, True),
    ("cISC4BudgetSimulator", "cFC4ISC4BudgetSimulator", FC4, True),
    ("cISC4DepartmentBudget", "cFC4ISC4DepartmentBudget", FC4, True),
    ("cIGZDate", "cFC4IGZDate", FC4, True),
    ("cIGZVariant", "cFC4IGZVariant", FC4, True),
    ("cISCPropertyHolder", "cFC4ISCPropertyHolder", FC4, True),
    ("cISC4LotManager", "cFC4ISC4LotManager", FC4, True),
    ("cISC4Lot", "cFC4ISC4Lot", FC4, True),
    ("cISC4LineItem", "cISC4LineItem", "cISC4LineItem.h", True),
    ("cIGZString", "cIGZString", "cIGZString.h", True),
    ("cIGZPersistResourceManager", "cIGZPersistResourceManager", "cIGZPersistResourceManager.h", True),
    ("cISC4BuildingDevelopmentSimulator", "cISC4BuildingDevelopmentSimulator", "cISC4BuildingDevelopmentSimulator.h", True),
    ("cISC4DisasterManager", "cISC4DisasterManager", "cISC4DisasterManager.h", True),
    ("cISC4HistoryWarehouse", "cISC4HistoryWarehouse", "cISC4HistoryWarehouse.h", True),
    ("cISC4NeighborsSimulator", "cFC4ISC4NeighborsSimulator", FC4, True),
    ("cISC4OrdinanceSimulator", "cISC4OrdinanceSimulator", "cISC4OrdinanceSimulator.h", True),
    ("cISCResExemplar", "cISCResExemplar", "cISCResExemplar.h", True),
    # Not in interfaces.tsv: checked against the implementing class's vtable in
    # docs/abi/vtables.tsv ("vt:<vtable name>"); the class may have more slots.
    ("vt:cSC4City::vftable", "cISC4City", "cISC4City.h", True),
    ("vt:cSC4App::vftable_3", "cISC4App", "cISC4App.h", True),
    ("vt:cSC4ResidentialSimulator::vftable", "cFC4ISC4ResidentialSimulator", FC4, True),
    # The residential buzzards (REH 4.7 step 3, RG1): the effects manager's slot 7 and the
    # visual effect's slots 2, 3 and 8.
    ("cISC4EffectsManager", "cISC4EffectsManager", "cISC4EffectsManager.h", True),
    ("cISC4VisualEffect", "cISC4VisualEffect", "cISC4VisualEffect.h", True),
    # Demand (docs/specs/demand.md): the simulator and the nodes are implemented;
    # the services of §14 that the plugin calls.
    ("vt:cSC4DemandSimulator::vftable", "cFC4ISC4DemandSimulator", FC4, True),
    ("vt:cSC4DemandSimulator::vftable_2", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cISC4Demand", "cFC4ISC4Demand", FC4, True),
    ("cISC4RegionalCity", "cFC4ISC4RegionalCity", FC4, True),
    ("vt:cSC4Nation::vftable", "cISC4Nation", "cISC4Nation.h", True),
    ("cISC4NeighborConnection", "cISC4NeighborConnection", "cISC4NeighborConnection.h", True),
    ("cISC4TrafficSimulator", "cISC4TrafficSimulator", "cISC4TrafficSimulator.h", True),
    ("vt:cGZPersistResourceKeyList::vftable", "cIGZPersistResourceKeyList", "cIGZPersistResourceKeyList.h", True),
    # Land value (docs/specs/landvalue.md §11, §12): implemented, and the game
    # services and grids it calls. A template instance is named Class<T>.
    ("cISC4LandValueSimulator", "cISC4LandValueSimulator", "cISC4LandValueSimulator.h", True),
    ("cIGZSerializable(LandValue)", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cIGZMessageTarget2(LandValue)", "cIGZMessageTarget2", "cIGZMessageTarget2.h", True),
    ("cISC4SimGrid<uint8>", "cISC4SimGrid<uint8_t>", "cISC4SimGrid.h", True),
    ("cISC4SimGrid<int16>", "cISC4SimGrid<int16_t>", "cISC4SimGrid.h", True),
    ("cISC4SimGrid<float>", "cISC4SimGrid<float>", "cISC4SimGrid.h", True),
    ("cISTETerrain", "cISTETerrain", "cISTETerrain.h", True),
    # The terrain shadow (the glue16 lane): surface water slots 6, 8 and 12
    # (the compiled declaration: the SDK's IsNavigable overloads land swapped).
    ("cISTESurfaceWater", "cFC4ISTESurfaceWater", FC4, True),
    ("cISC4ZoneManager", "cISC4ZoneManager", "cISC4ZoneManager.h", True),
    ("cISC4LotDeveloper", "cISC4LotDeveloper", "cISC4LotDeveloper.h", True),
    ("cISC4GrowthDeveloper", "cISC4GrowthDeveloper", "cISC4GrowthDeveloper.h", True),
    # Weather (docs/specs/weather.md): implemented, and the region it reads.
    ("cISC4WeatherSimulator", "cFC4ISC4WeatherSimulator", FC4, True),
    ("vt:cSC4WeatherSimulator::vftable_3", "cIGZSerializable", "cIGZSerializable.h", True),
    ("vt:cSC4Region::vftable", "cISC4Region", "cISC4Region.h", True),
    # Ordinances (docs/specs/ordinance.md): the simulator (cISC4OrdinanceSimulator
    # above) and the ordinance objects are implemented; the city services the
    # standard ordinances read (§3.6).
    ("cISC4Ordinance", "cFC4ISC4Ordinance", FC4, True),
    ("vt:cSC4OrdinanceSimulator::vftable_2", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cISC4LandfillDeveloper", "cISC4LandfillDeveloper", "cISC4LandfillDeveloper.h", True),
    # (the SDK police simulator is reported only: cFC4ISC4PoliceSimulator below)
    ("cISC4PoliceSimulator", "cISC4PoliceSimulator", "cISC4PoliceSimulator.h", False),
    ("cISC4PowerSimulator", "cISC4PowerSimulator", "cISC4PowerSimulator.h", True),
    ("cISC4PollutionSimulator", "cISC4PollutionSimulator", "cISC4PollutionSimulator.h", True),
    # Power (docs/specs/power.md §10, §11.1): implemented, and the services it calls.
    ("cIGZSerializable(Power)", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cIGZMessageTarget2(Power)", "cIGZMessageTarget2", "cIGZMessageTarget2.h", True),
    ("cISC4Occupant", "cISC4Occupant", "cISC4Occupant.h", True),
    ("cISC4OccupantManager", "cISC4OccupantManager", "cISC4OccupantManager.h", True),
    ("cISC4BuildingOccupant", "cISC4BuildingOccupant", "cISC4BuildingOccupant.h", True),
    ("cISC4NetworkOccupant", "cISC4NetworkOccupant", "cISC4NetworkOccupant.h", True),
    ("cISC4NetworkManager", "cISC4NetworkManager", "cISC4NetworkManager.h", True),
    ("cISC4NetworkTool(PowerLine)", "cFC4ISC4NetworkTool", FC4, True),
    # The network tool (glue29): the replacement implements the compiled order
    # (cFC4ISC4NetworkToolFull in FC4Interfaces.h, slots 0-50). The power-line
    # row above stays: the power simulator's 3-10 view of the same vtable.
    ("cISC4NetworkTool", "cFC4ISC4NetworkToolFull", FC4, True),
    ("cISC4NeighborDeal", "cISC4NeighborDeal", "cISC4NeighborDeal.h", True),
    # Neighbours (docs/specs/neighbors.md section 4.2): the connection's traffic source.
    ("cISC4TrafficSource", "cISC4TrafficSource", "cISC4TrafficSource.h", True),
    ("cISC4View3DWin", "cISC4View3DWin", "cISC4View3DWin.h", True),
    ("cISC4PowerLineOccupant", "cISC4PowerLineOccupant", "cISC4PowerLineOccupant.h", True),
    ("cISC4PowerPoleOccupant", "cFC4ISC4PowerPoleOccupant", FC4, True),
    ("cISC4PowerSimulator", "cFC4ISC4PowerSimulator", FC4, True),
    ("cISC4Demolition", "cISC4Demolition", "cISC4Demolition.h", True),
    # Plumbing (docs/specs/plumbing.md §10): implemented, and the sub-object
    # interfaces of its own vtables.
    ("cISC4PlumbingSimulator", "cFC4ISC4PlumbingSimulator", FC4, True),
    ("cIGZSerializable(Plumbing)", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cIGZMessageTarget2(Plumbing)", "cIGZMessageTarget2", "cIGZMessageTarget2.h", True),
    # Pollution (docs/specs/pollution.md §8): implemented (the SDK row above
    # stays for the plumbing glue's calls), its sub-object interfaces, and the
    # prop developer it registers with.
    ("cISC4PollutionSimulator", "cFC4ISC4PollutionSimulator", FC4, True),
    ("cIGZSerializable(Pollution)", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cIGZMessageTarget2(Pollution)", "cIGZMessageTarget2", "cIGZMessageTarget2.h", True),
    ("cISC4PropRequester(Pollution)", "cFC4ISC4PropRequester", FC4, True),
    # The SDK declares slots 0-10, which match prop-developer.md section 8.1; slot 11 is the
    # class's scalar deleting destructor (the trailing deleting destructor rule in abi_conformance_test.cpp).
    ("cISC4PropDeveloper", "cISC4PropDeveloper", "cISC4PropDeveloper.h", True),
    # Flammability (docs/specs/flammability.md §11, §12): implemented, and the
    # services it calls. The SDK declaration of the simulator matches docs/abi.
    ("cISC4FlammabilitySimulator", "cISC4FlammabilitySimulator", "cISC4FlammabilitySimulator.h", True),
    ("cIGZSerializable(Flammability)", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cIGZMessageTarget2(Flammability)", "cIGZMessageTarget2", "cIGZMessageTarget2.h", True),
    ("cISC4DisasterManagerFire", "cISC4DisasterManagerFire", "cISC4DisasterManagerFire.h", True),
    ("cISC4DisasterLayer", "cISC4DisasterLayer", "cISC4DisasterLayer.h", True),
    # The tutorial system (tutorial.md section 6): the SDK declaration (slots 0-17, which the
    # flammability glue calls) extended by the three slots the SDK header lacks (18-20).
    ("cISC4TutorialSystem", "cFC4ISC4TutorialSystem", FC4, True),
    # Protection (docs/specs/protection.md §14, §15): both simulators, the
    # sub-object interfaces of their own vtables, and the services they call.
    ("cISC4FireProtectionSimulator", "cFC4ISC4FireProtectionSimulator", FC4, True),
    ("cISC4PoliceSimulator", "cFC4ISC4PoliceSimulator", FC4, True),
    ("cISC4DispatchSource(Fire)", "cFC4ISC4DispatchSource", FC4, True),
    ("cISC4DispatchSource(Police)", "cFC4ISC4DispatchSource", FC4, True),
    ("cIGZSerializable(Fire)", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cIGZSerializable(Police)", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cIGZMessageTarget2(Fire)", "cIGZMessageTarget2", "cIGZMessageTarget2.h", True),
    ("cIGZMessageTarget2(Police)", "cIGZMessageTarget2", "cIGZMessageTarget2.h", True),
    ("vt:cSC4DispatchManager::vftable", "cFC4ISC4DispatchManager", FC4, True),
    ("vt:cSC4AutomataControllerManager::vftable", "cISC4AutomataControllerManager",
     "cISC4AutomataControllerManager.h", True),
    # Crime (docs/specs/crime.md section 1, sections 11 and 13): implemented, the sub-object
    # interfaces of its own vtables, and the services it calls (police through
    # cFC4ISC4PoliceSimulator above).
    ("cISC4CrimeSimulator", "cFC4ISC4CrimeSimulator", FC4C, True),
    ("cIGZSerializable(Crime)", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cIGZMessageTarget2(Crime)", "cIGZMessageTarget2", "cIGZMessageTarget2.h", True),
    ("cISC4PropRequester(Crime)", "cFC4CrimePropRequester", FC4C, True),
    ("cISC4PropManager", "cISC4PropManager", "cISC4PropManager.h", True),
    ("cISC4PropOccupant", "cISC4PropOccupant", "cISC4PropOccupant.h", True),
    # Aura (docs/specs/aura.md section 12): implemented (the SDK declaration;
    # MSVC's overload reversal gives the AddTransientEffect order of 12.1),
    # the sub-object interfaces of its own vtables, and the grids it creates.
    ("cISC4AuraSimulator", "cISC4AuraSimulator", "cISC4AuraSimulator.h", True),
    ("cIGZSerializable(Aura)", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cIGZMessageTarget2(Aura)", "cIGZMessageTarget2", "cIGZMessageTarget2.h", True),
    ("cISC4SimGrid<int8>", "cISC4SimGrid<int8_t>", "cISC4SimGrid.h", True),
    ("cISC4SimGrid<uint16>", "cISC4SimGrid<uint16_t>", "cISC4SimGrid.h", True),
    ("cISC4OrdinanceSimulator", "cFC4ISC4OrdinanceSimulator", FC4, True),
    # Flora (docs/specs/flora.md section 11): implemented (the SDK declaration;
    # MSVC's overload reversal gives the game's order of slots 5/6, 8/9 and
    # 17/18), the sub-object interfaces of its own vtable, and the services it
    # calls that are not listed above.
    ("cISC4FloraSimulator", "cISC4FloraSimulator", "cISC4FloraSimulator.h", True),
    ("cIGZSerializable(Flora)", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cIGZMessageTarget2(Flora)", "cIGZMessageTarget2", "cIGZMessageTarget2.h", True),
    ("cISC4FloraOccupant", "cISC4FloraOccupant", "cISC4FloraOccupant.h", True),
    ("cIGZCheatCodeManager", "cIGZCheatCodeManager", "cIGZCheatCodeManager.h", True),
    ("cIGZCommandServer", "cIGZCommandServer", "cIGZCommandServer.h", True),
    # Landfill (docs/specs/landfill.md section 7): implemented through the
    # compiled slot order (the SDK row above stays for the pollution glue's
    # calls), and the services it calls that are not listed above.
    ("cISC4LandfillDeveloper", "cFC4ISC4LandfillDeveloper", FC4, True),
    ("cISC4LandfillOccupant", "cISC4LandfillOccupant", "cISC4LandfillOccupant.h", True),
    ("cISC4TrafficNetworkMap", "cFC4ISC4TrafficNetworkMap", FC4, True),
    # (landfill.md 4.8.1, 4.8.4, 4.8.5: the tiles' exemplar holder, the
    # landfill ground texture, and GZCOM, which creates both)
    ("cISCExemplarPropertyHolder", "cISCExemplarPropertyHolder", "cISCExemplarPropertyHolder.h", True),
    ("vt:cSC4LotBaseTextureOccupant::vftable", "cISC4LotBaseTextureOccupant", "cISC4LotBaseTextureOccupant.h", True),
    ("vt:cGZCOM::vftable", "cIGZCOM", "cIGZCOM.h", True),
    # Residential (docs/specs/residential.md section 10): implemented through the
    # compiled slot order (the vt: row above also checks the class slots), and
    # the sub-object interfaces of its own vtables.
    ("cISC4ResidentialSimulator", "cFC4ISC4ResidentialSimulator", FC4, True),
    ("cIGZSerializable(Residential)", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cIGZMessageTarget2(Residential)", "cIGZMessageTarget2", "cIGZMessageTarget2.h", True),
    # Development (docs/specs/building-development.md section 7, growth-developer.md):
    # the simulator implemented through the compiled slot order (the SDK row
    # above stays for the other glue's calls), the sub-object interfaces of its
    # own vtables, the growth developers it calls, and the services it calls
    # that are not listed above (the lot configuration filter is implemented).
    ("cISC4BuildingDevelopmentSimulator", "cFC4ISC4BuildingDevelopmentSimulator", FCD, True),
    ("cIGZMessageTarget(BuildingDevelopment)", "cIGZMessageTarget", "cIGZMessageTarget.h", True),
    ("cIGZMessageTarget2(BuildingDevelopment)", "cIGZMessageTarget2", "cIGZMessageTarget2.h", True),
    ("cIGZSerializable(BuildingDevelopment)", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cISC4GrowthDeveloper", "cFC4ISC4GrowthDeveloper", FCD, True),
    ("cIGZMessageTarget2(GrowthDeveloper)", "cIGZMessageTarget2", "cIGZMessageTarget2.h", True),
    ("cIGZSerializable(GrowthDeveloper)", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cISC4TractDeveloper", "cFC4ISC4TractDeveloper", FCD, True),
    ("cISC4CommercialSimulator", "cISC4CommercialSimulator", "cISC4CommercialSimulator.h", True),
    ("cISC4LotConfiguration", "cISC4LotConfiguration", "cISC4LotConfiguration.h", True),
    ("cISC4LotConfigurationFilter", "cISC4LotConfigurationFilter", "cISC4LotConfigurationFilter.h", True),
    ("cISC4LotConfigurationManager", "cISC4LotConfigurationManager", "cISC4LotConfigurationManager.h", True),
    ("cISC4AdvisorSystem", "cISC4AdvisorSystem", "cISC4AdvisorSystem.h", True),
    ("cISC4Advisor", "cISC4Advisor", "cISC4Advisor.h", True),
    ("cIGZCommandParameterSet", "cIGZCommandParameterSet", "cIGZCommandParameterSet.h", True),
    # The parameter set as compiled (slots 3-10; slot 10 takes a variant): the
    # SDK row above stays for the building development and flora glue's calls.
    ("cIGZCommandParameterSet", "cFC4IGZCommandParameterSet", FC4, True),
    ("cIGZMessageServer2", "cIGZMessageServer2", "cIGZMessageServer2.h", True),
    ("cISC4DemandSimulator", "cISC4DemandSimulator", "cISC4DemandSimulator.h", True),
    # Zones (docs/specs/zones.md section 11): the zone manager, implemented through
    # the compiled slot order (the SDK row above stays for other glue's calls),
    # its sub-object interfaces, and the zone developer and lot configurations
    # it calls.
    ("cISC4ZoneManager", "cFC4ISC4ZoneManager", FC4, True),
    ("cIGZSerializable(ZoneManager)", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cIGZMessageTarget2(ZoneManager)", "cIGZMessageTarget2", "cIGZMessageTarget2.h", True),
    ("cISC4ZoneDeveloper", "cISC4ZoneDeveloper", "cISC4ZoneDeveloper.h", True),
    # Traffic (docs/specs/traffic-simulator.md section 7, pathfinder.md): the simulator through
    # the compiled slot order (the SDK row above stays for the demand glue's call), its
    # serializable, the pathfinders of slot 6, and the u32 grids it creates.
    ("cISC4TrafficSimulator", "cFC4ISC4TrafficSimulator", FC4, True),
    ("vt:cSC4TrafficSimulator::vftable_2", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cISC4PathFinder", "cFC4ISC4PathFinder", FC4, True),
    ("cISC4SimGrid<uint32>", "cISC4SimGrid<uint32_t>", "cISC4SimGrid.h", True),
    # History (docs/specs/history.md section 8): the warehouse implemented through the
    # compiled slot order (the SDK row above stays for the other glue's calls; its
    # Dispose overloads and Coalesce are waived), and its serializable.
    ("cISC4HistoryWarehouse", "cFC4ISC4HistoryWarehouse", FC4, True),
    ("vt:cSC4HistoryWarehouse::vftable_2", "cIGZSerializable", "cIGZSerializable.h", True),
    # The 24-hour clock (docs/specs/clock24.md section 8): the compiled slot
    # order (slots 7, 9, 19 and 20 differ from the SDK), its serializable
    # and its system service.
    ("cISC424HourClock", "cFC4ISC424HourClock", FC4, True),
    ("vt:cSC424HourClock::vftable_2", "cIGZSerializable", "cIGZSerializable.h", True),
    ("vt:cSC424HourClock::vftable_3", "cIGZSystemService", "cIGZSystemService.h", True),
    # Crime type names (crime.md section 7 step 3; lua-host.md section 10.1).
    ("cISCStringDetokenizer", "cFC4ISCStringDetokenizer", FC4C, True),
    # The tract developer (docs/specs/tract-developer.md section 8): the lot
    # configuration manager through the compiled slot order (slot 9's set is
    # the game's hash_set; the SDK row above stays for other glue's calls).
    ("cISC4LotConfigurationManager", "cFC4ISC4LotConfigurationManager", FC4T, True),
    # The lot developer (docs/specs/lot-developer.md section 11): implemented through
    # the compiled slot order (the SDK row above stays for other glue's calls), its
    # sub-object interfaces, and the animation-tick manager it registers with.
    ("cISC4LotDeveloper", "cFC4ISC4LotDeveloper", FCL, True),
    ("cIGZMessageTarget2(LotDeveloper)", "cIGZMessageTarget2", "cIGZMessageTarget2.h", True),
    ("cIGZSerializable(LotDeveloper)", "cIGZSerializable", "cIGZSerializable.h", True),
    ("vt:cSC4AnimationTickManager::vftable", "cFC4ISC4AnimationTickManager", FCL, True),
    ("cISC4NetworkManager", "cFC4ISC4NetworkManager", FCL, True),
    # The dialog closer (docs/specs/modal-dialogs.md section 3.2; tooling switch
    # close_dialogs): the window manager and the window through the compiled
    # slot order (the SDK headers are wrong for these slots), the legacy message
    # server it subscribes to, and the message target it implements.
    ("cIGZWinMgr", "cFC4IGZWinMgr", FCW, True),
    ("cIGZWin", "cFC4IGZWin", FCW, True),
    ("cIGZMessageServer", "cFC4IGZMessageServer", FCW, True),
    # The landfill zot's view rotation (landfill.md section 4.7.5, LG21):
    # cISC43DRender through slot 8 GetViewUtilities.
    ("vt:cSC43DRender::vftable", "cFC4ISC43DRender", FCW, True),
    ("cIGZMessageTarget(AudioEventHandler)", "cIGZMessageTarget", "cIGZMessageTarget.h", True),
    # The commercial and industrial simulators (docs/specs/commercial-industrial.md section 9):
    # implemented through the SDK declarations (cISC4CommercialSimulator is listed above
    # for the growth developers' calls).
    ("cISC4IndustrialSimulator", "cISC4IndustrialSimulator", "cISC4IndustrialSimulator.h", True),
    # The airport and seaport developers (docs/specs/port-developers.md section 8; the
    # glue12b2 lane): implemented and called through the compiled slot order (the SDK's
    # GetNumSeaports takes an argument; GetAvgEfficiency's hidden result), and their
    # message-target and serializable sub-objects. The port sub-object at +4 implements
    # slots 0-9 only (slots 10-18 are internal, section 8.3): not listed.
    ("cISC4AirportDeveloper", "cFC4ISC4AirportDeveloper", FCP, True),
    ("cISC4SeaportDeveloper", "cFC4ISC4SeaportDeveloper", FCP, True),
    ("cIGZMessageTarget2(AirportDeveloper)", "cIGZMessageTarget2", "cIGZMessageTarget2.h", True),
    ("cIGZMessageTarget2(SeaportDeveloper)", "cIGZMessageTarget2", "cIGZMessageTarget2.h", True),
    ("cIGZSerializable(AirportDeveloper)", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cIGZSerializable(SeaportDeveloper)", "cIGZSerializable", "cIGZSerializable.h", True),
    # The SDK declarations we replaced, and interfaces the plugin does not use yet: reported only.
    ("cISC4Simulator", "cISC4Simulator", "cISC4Simulator.h", False),
    ("cISC4BudgetSimulator", "cISC4BudgetSimulator", "cISC4BudgetSimulator.h", False),
    ("cISC4DepartmentBudget", "cISC4DepartmentBudget", "cISC4DepartmentBudget.h", False),
    ("cIGZDate", "cIGZDate", "cIGZDate.h", False),
    ("cIGZVariant", "cIGZVariant", "cIGZVariant.h", False),
    ("cISCPropertyHolder", "cISCPropertyHolder", "cISCPropertyHolder.h", False),
    ("cISC4LotManager", "cISC4LotManager", "cISC4LotManager.h", False),
    ("cISC4Lot", "cISC4Lot", "cISC4Lot.h", False),
    ("cISC4NeighborsSimulator", "cISC4NeighborsSimulator", "cISC4NeighborsSimulator.h", False),
    ("cISC4Ordinance", "cISC4Ordinance", "cISC4Ordinance.h", False),
    ("cISC4RegionalCity", "cISC4RegionalCity", "cISC4RegionalCity.h", False),
    ("vt:cSC4ResidentialSimulator::vftable", "cISC4ResidentialSimulator", "cISC4ResidentialSimulator.h", False),
    ("vt:cSC4DemandSimulator::vftable", "cISC4DemandSimulator", "cISC4DemandSimulator.h", False),
    ("cISC4Demand", "cISC4Demand", "cISC4Demand.h", False),
    ("cISC4WeatherSimulator", "cISC4WeatherSimulator", "cISC4WeatherSimulator.h", False),
    ("cISC4GameDataRegistry", "cISC4GameDataRegistry", "cISC4GameDataRegistry.h", False),
    ("cISC4NetworkTool(PowerLine)", "cISC4NetworkTool", "cISC4NetworkTool.h", False),
    # The cohort chain behind an exemplar (data-formats §4.4): FC4Properties.cpp EffectiveProperties.
    ("cISCResExemplarCohort", "cISCResExemplarCohort", "cISCResExemplarCohort.h", True),
    # Pole movement: validate the corrected x87 width getter on the terrain
    # prefix. The SDK row stays for the other terrain calls; its uint32 getter
    # remains unused (network-occupants 4.7; terrain slot 6).
    ("vt:cSTETerrain::vftable", "cFC4ISTETerrainCellWidth", FC4, True),
]

BASE_HEADERS = {"cIGZSerializable": "cIGZSerializable.h",
                # the port developers (glue12b2): their shared base
                "cFC4ISC4PortDeveloper": FCP}


def strip_comments(text):
    text = re.sub(r"/\*.*?\*/", " ", text, flags=re.S)
    return re.sub(r"//[^\n]*", " ", text)


def class_body(path, cls):
    cls = cls.split("<")[0]  # a template instance: the template's declaration
    if not os.path.isabs(path):
        path = os.path.join(SDK, path)
    text = strip_comments(open(path, encoding="utf-8", errors="replace").read())
    m = re.search(r"\bclass\s+%s\s*(?::\s*public\s+(\w+))?\s*\{" % re.escape(cls), text)
    if m is None:
        sys.exit("class %s not found in %s" % (cls, path))
    depth, i = 1, m.end()
    while depth:
        depth += {"{": 1, "}": -1}.get(text[i], 0)
        i += 1
    return m.group(1), text[m.end():i - 1]


def split_params(params):
    out, depth, cur = [], 0, ""
    for ch in params:
        if ch in "(<":
            depth += 1
        elif ch in ")>":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(cur)
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur)
    return [p.split("=")[0].strip() for p in out]


def methods(path, cls):
    base, body = class_body(path, cls)
    found = []
    if base and base != "cIGZUnknown":
        found += methods(BASE_HEADERS.get(base, base + ".h"), base)
    # Nested types used in signatures need the class prefix at namespace scope.
    nested = set(re.findall(r"\b(?:class|struct|enum(?:\s+class)?)\s+(\w+)", body))
    nested |= set(re.findall(r"\busing\s+(\w+)\s*=", body))
    nested |= set(re.findall(r"\btypedef\b[^;(]*?\b(\w+)\s*;", body))
    nested |= set(re.findall(r"\btypedef\b[^;]*?\(\s*\*\s*(\w+)\s*\)", body))
    nested -= {"class", "struct", "enum"}
    decl = re.compile(r"virtual\s+([^;{]*?)\b(operator\s*[^\s(]+|\w+)\s*\(([^;{]*?)\)\s*(const)?\s*(?:override)?\s*=\s*0\s*;",
                      re.S)
    for m in decl.finditer(body):
        ret, name, params, const = (" ".join(m.group(1).split()), m.group(2), " ".join(m.group(3).split()),
                                    m.group(4) or "")
        if name.startswith("~"):
            continue

        def qualify(t, cls=cls):
            return re.sub(r"\b(%s)\b" % "|".join(map(re.escape, nested)), cls + r"::\1", t) if nested else t

        ps = ", ".join(qualify(p) for p in split_params(params) if p != "void")
        found.append((name, qualify(ret), ps, const))
    return found


def main():
    out = ["// SPDX-License-Identifier: GPL-3.0-or-later", "//"]
    out.append("// Generated by tests/gen_abi_entries.py - do not edit. Regenerate after adding")
    out.append("// an interface or a method (a gap in the slots or a compile error says so).")
    table = []
    for i, (iface, cls, header, required) in enumerate(INTERFACES):
        ms = [("QueryInterface", "bool", "uint32_t riid, void** ppvObj", ""), ("AddRef", "uint32_t", "", ""),
              ("Release", "uint32_t", "", "")] + methods(header, cls)
        counts = {}
        for name, _, _, _ in ms:
            counts[name] = counts.get(name, 0) + 1
        fn = "AbiCollect%d_%s" % (i, re.sub(r"\W", "_", cls))
        out.append("")
        out.append("void %s(std::vector<AbiMethod>& m) {" % fn)
        out.append("    using C = %s;" % cls)
        for name, ret, ps, const in ms:
            if counts[name] == 1:
                out.append("    m.push_back(AbiM(&C::%s, \"%s\"));" % (name, name))
            else:
                sig = "%s(%s)%s" % (name, ps, " const" if const else "")
                out.append("    m.push_back(AbiM(static_cast<%s (C::*)(%s)%s>(&C::%s), \"%s\"));"
                           % (ret, ps, " const" if const else "", name, sig.replace('"', '\\"')))
        out.append("}")
        table.append("    {\"%s\", \"%s\", %s, &%s}," % (iface, cls, "true" if required else "false", fn))
    out.append("")
    out.append("const AbiInterface kAbiInterfaces[] = {")
    out += table
    out.append("};")
    path = os.path.join(HERE, "abi_entries.inc")
    body = "\n".join(out) + "\n"
    # Only rewrite when the content changed. CMake regenerates this file on
    # every build (it is a build product, not checked in), so rewriting
    # unconditionally would move its mtime and recompile the conformance test
    # every time for nothing.
    old = None
    if os.path.isfile(path):
        with open(path, "r", encoding="utf-8", newline="") as f:
            old = f.read()
    if old == body:
        print("unchanged %s (%d interfaces)" % (path, len(INTERFACES)))
    else:
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            f.write(body)
        print("wrote %s (%d interfaces)" % (path, len(INTERFACES)))


if __name__ == "__main__":
    main()

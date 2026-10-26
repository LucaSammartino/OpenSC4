-- Candidate scenario; only the isolated game-slot copy may change.
-- Requires the opt-in lua_script harness key. No command is calibrated here.
sc4app.SetScriptAutoYield(-1, true)
while sc4app.GetAppState() ~= 2 do
    sc4app.GameDelay('frames', 1)
end
sc4app.SetPopupModalDialogsEnabled(false)
-- Preflight a vacant dry area covering cells 12..16 on both axes.
local altitude = sc4app.GetCellAltitude(12, 12)
sc4app.SetCellAltitude(12, 12, 16, 16, altitude + 2)
sc4app.GameDelay('frames', 120)
-- This flattens a small rectangle; the entire slot copy is disposable.
sc4app.SetCellAltitude(12, 12, 16, 16, altitude)
local sea = sc4app.GetSeaLevel()
sc4app.SetSeaLevel(sea + 1)
sc4app.GameDelay('frames', 120)
sc4app.SetSeaLevel(sea)
-- Restoring one height does not restore the original per-cell terrain shape.
sc4app.GameDelay('frames', 60)

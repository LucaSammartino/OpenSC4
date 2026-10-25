-- Candidate scenario; only the isolated game-slot copy may change.
-- Requires the opt-in lua_script harness key. No command is calibrated here.
sc4app.SetScriptAutoYield(-1, true)
while sc4app.GetAppState() ~= 2 do
    sc4app.GameDelay('frames', 1)
end
sc4app.SetPopupModalDialogsEnabled(false)
sc4app.GetCityName()
sc4app.GetCitySize()
sc4app.GetSimulationDate()
sc4app.GetDateAndTime()
sc4app.GetTimerTime()
sc4app.GetFrameCount()
sc4app.GetFrameRate()
sc4app.GetOccupantCount()
sc4app.GetMemInfo()
sc4app.GetViewTarget()
sc4app.GameDelay('frames', 60)
-- Returning from a query is distinct from validating its returned values.
sc4app.GameDelay('frames', 60)

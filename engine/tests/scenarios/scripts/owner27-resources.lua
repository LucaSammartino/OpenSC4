-- Candidate scenario; only the isolated game-slot copy may change.
-- Requires the opt-in lua_script harness key. No command is calibrated here.
sc4app.SetScriptAutoYield(-1, true)
while sc4app.GetAppState() ~= 2 do
    sc4app.GameDelay('frames', 1)
end
sc4app.SetPopupModalDialogsEnabled(false)
-- Start observation before launch to include package and city loading.
sc4app.GetCityName()
sc4app.GetOccupantCount()
sc4app.SaveCity(true)
sc4app.GameDelay('frames', 120)
-- Binary loading and saving do not prove text import or editor activation.
sc4app.GameDelay('frames', 60)

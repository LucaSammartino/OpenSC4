-- Candidate scenario; only the isolated game-slot copy may change.
-- Requires the opt-in lua_script harness key. No command is calibrated here.
sc4app.SetScriptAutoYield(-1, true)
while sc4app.GetAppState() ~= 2 do
    sc4app.GameDelay('frames', 1)
end
sc4app.SetPopupModalDialogsEnabled(false)
sc4app.SaveCity(true)
sc4app.GameDelay('frames', 60)
sc4app.QuitCity(false)
while sc4app.GetAppState() ~= 1 do
    sc4app.GameDelay('frames', 1)
end
sc4app.LoadCity('North Weburg', 'any', 'full')
while sc4app.GetAppState() ~= 2 do
    sc4app.GameDelay('frames', 1)
end
-- Verify that Lua continues across the transition and the new city is loaded.
sc4app.GameDelay('frames', 60)

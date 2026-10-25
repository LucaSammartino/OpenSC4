-- Candidate scenario; only the isolated game-slot copy may change.
-- Requires the opt-in lua_script harness key. No command is calibrated here.
sc4app.SetScriptAutoYield(-1, true)
while sc4app.GetAppState() ~= 2 do
    sc4app.GameDelay('frames', 1)
end
sc4app.SetPopupModalDialogsEnabled(false)
sc4app.ToggleMySimPanel()
sc4app.GameDelay('frames', 120)
-- Add a resident to an occupied home using a calibrated panel/map driver.
-- Inspect balloons, follow a trip and separately test compatible import data.
sc4app.Cancel()
sc4app.GameDelay('frames', 60)

-- Candidate scenario; only the isolated game-slot copy may change.
-- Requires the opt-in lua_script harness key. No command is calibrated here.
sc4app.SetScriptAutoYield(-1, true)
while sc4app.GetAppState() ~= 2 do
    sc4app.GameDelay('frames', 1)
end
sc4app.SetPopupModalDialogsEnabled(false)
-- Select the hardware backend in the launch profile before this script.
sc4app.Zoom0()
sc4app.GameDelay('frames', 60)
sc4app.Zoom5()
sc4app.GameDelay('frames', 60)
sc4app.EnableNightOnly()
sc4app.GameDelay('frames', 60)
sc4app.EnableDayAndNight()
sc4app.OpenGraphicOptionsDialog()
sc4app.GameDelay('frames', 30)
sc4app.Cancel()
-- Changing graphics options requires a calibrated dialog-control driver.
sc4app.GameDelay('frames', 60)

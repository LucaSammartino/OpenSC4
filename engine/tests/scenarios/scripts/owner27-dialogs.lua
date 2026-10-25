-- Candidate scenario; only the isolated game-slot copy may change.
-- Requires the opt-in lua_script harness key. No command is calibrated here.
sc4app.SetScriptAutoYield(-1, true)
while sc4app.GetAppState() ~= 2 do
    sc4app.GameDelay('frames', 1)
end
sc4app.SetPopupModalDialogsEnabled(false)
sc4app.OpenAudioOptionsDialog()
sc4app.GameDelay('frames', 30)
sc4app.Cancel()
sc4app.OpenGraphicOptionsDialog()
sc4app.GameDelay('frames', 30)
sc4app.Cancel()
sc4app.OpenPlayOptionsDialog()
sc4app.GameDelay('frames', 30)
sc4app.Cancel()
sc4app.OpenPhotoAlbumDialog()
sc4app.GameDelay('frames', 30)
sc4app.Cancel()
-- Advisor panels and list/tree interaction still need calibrated controls.
sc4app.GameDelay('frames', 60)

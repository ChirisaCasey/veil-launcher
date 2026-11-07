; Veil Launcher NSIS installer hooks.
; On uninstall, restore the original VALORANT menu video and remove the
; background scheduled task / autostart entry before files are deleted.

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "Restoring original VALORANT menu video..."
  nsExec::ExecToLog '"$INSTDIR\veil-launcher.exe" --cleanup'
  Pop $0
!macroend

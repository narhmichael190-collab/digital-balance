; Liba Bilancia Windows branding migration.
; Remove shortcuts left behind by older Digital Balance installers.

!macro NSIS_HOOK_PREINSTALL
  Delete "$SMPROGRAMS\Digital Balance.lnk"
  Delete "$SMPROGRAMS\Digital Balance\Digital Balance.lnk"
  RMDir "$SMPROGRAMS\Digital Balance"
  Delete "$DESKTOP\Digital Balance.lnk"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  Delete "$SMPROGRAMS\Digital Balance.lnk"
  Delete "$SMPROGRAMS\Digital Balance\Digital Balance.lnk"
  RMDir "$SMPROGRAMS\Digital Balance"
  Delete "$DESKTOP\Digital Balance.lnk"
!macroend

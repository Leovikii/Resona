; SPDX-License-Identifier: GPL-3.0-only

!macro NSIS_HOOK_PREINSTALL
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Tauri owns the association lifecycle. Resona only replaces each ProgID icon
  ; with its branded format icon after the official association macro completes.
  WriteRegStr SHCTX "Software\Classes\Resona.MP3\DefaultIcon" "" "$\"$INSTDIR\icons\file-mp3.ico$\",0"
  WriteRegStr SHCTX "Software\Classes\Resona.WAV\DefaultIcon" "" "$\"$INSTDIR\icons\file-wav.ico$\",0"
  WriteRegStr SHCTX "Software\Classes\Resona.FLAC\DefaultIcon" "" "$\"$INSTDIR\icons\file-flac.ico$\",0"
  !insertmacro UPDATEFILEASSOC
!macroend

!macro NSIS_HOOK_PREUNINSTALL
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Updater runs the old uninstaller with /UPDATE and must preserve all state.
  ${If} $UpdateMode <> 1
    DeleteRegKey HKCU "Software\io.github.vki.resona"
    DeleteRegKey HKCU "Software\Resona"
    ; The customized official template also removes both app-data locations.
    ; These exact app-owned paths make the cleanup idempotent for old installers.
    SetShellVarContext current
    RMDir /r "$APPDATA\io.github.vki.resona"
    RMDir /r "$LOCALAPPDATA\io.github.vki.resona"
  ${EndIf}
!macroend

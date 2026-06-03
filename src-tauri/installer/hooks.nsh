; Custom NSIS hooks for Drift's installer, wired in via
; tauri.conf.json > bundle.windows.nsis.installerHooks.
;
; Tauri's NSIS template invokes these macros at fixed points around install and
; uninstall. We only need the pre-uninstall hook: offer to remove Drift's app
; data (settings + the persisted torrent list / resume data) so an uninstall can
; leave the machine clean. Downloaded files live wherever the user chose and are
; never touched.
;
; Drift stores its data in %APPDATA%\Drift (dirs::data_dir() == roaming AppData).

!macro NSIS_HOOK_PREINSTALL
!macroend

!macro NSIS_HOOK_POSTINSTALL
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Default to "No" for silent/automated uninstalls so we never delete settings
  ; without an explicit choice.
  MessageBox MB_YESNO|MB_ICONQUESTION "Also remove Drift's settings and torrent list?$\n$\nYour downloaded files are NOT affected." /SD IDNO IDNO drift_keep_appdata
    RMDir /r "$APPDATA\Drift"
  drift_keep_appdata:
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
!macroend

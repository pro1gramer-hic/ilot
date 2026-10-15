; Hooks of the NSIS installer (tauri.conf.json → bundle.windows.nsis.installerHooks).
; Before its files go, îlot unplugs every AI, removes its relay scripts and identity
; package, and turns off the start with Windows; the user chooses whether settings and the
; keys of the Credential Manager go too.

!macro NSIS_HOOK_PREUNINSTALL
  MessageBox MB_YESNO|MB_ICONQUESTION "Supprimer aussi tes réglages et les clés gardées par îlot ?$\n(Tes IA sont déconnectées et leurs configs rendues telles qu'elles étaient dans tous les cas.)" IDYES ilot_remove_data
  ExecWait '"$INSTDIR\ilot.exe" --uninstall-cleanup'
  Goto ilot_cleanup_done
  ilot_remove_data:
  ExecWait '"$INSTDIR\ilot.exe" --uninstall-cleanup --remove-data'
  ilot_cleanup_done:
!macroend

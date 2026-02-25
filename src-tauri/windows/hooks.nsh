; NSIS installer hooks for LazyRec
; Copy FFmpeg DLLs from resources/ to the install directory so Windows
; can find them at process startup (implicit DLL linking).

!macro NSIS_HOOK_POSTINSTALL
  ; FFmpeg shared libraries must be alongside the .exe for the OS loader.
  ; Tauri places bundled resources in $INSTDIR\resources\ but Windows DLL
  ; search only looks in the exe directory.
  CopyFiles /SILENT "$INSTDIR\resources\*.dll" "$INSTDIR"
!macroend

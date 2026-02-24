; NSIS installer hooks for LazyRec
; Copy FFmpeg DLLs from resources/ to the install directory so Windows
; can find them at process startup (implicit DLL linking).

!macro NSIS_HOOK_POSTINSTALL
  ; FFmpeg shared libraries must be alongside the .exe for the OS loader
  CopyFiles /SILENT "$INSTDIR\resources\avcodec-61.dll" "$INSTDIR\avcodec-61.dll"
  CopyFiles /SILENT "$INSTDIR\resources\avdevice-61.dll" "$INSTDIR\avdevice-61.dll"
  CopyFiles /SILENT "$INSTDIR\resources\avfilter-10.dll" "$INSTDIR\avfilter-10.dll"
  CopyFiles /SILENT "$INSTDIR\resources\avformat-61.dll" "$INSTDIR\avformat-61.dll"
  CopyFiles /SILENT "$INSTDIR\resources\avutil-59.dll" "$INSTDIR\avutil-59.dll"
  CopyFiles /SILENT "$INSTDIR\resources\swresample-5.dll" "$INSTDIR\swresample-5.dll"
  CopyFiles /SILENT "$INSTDIR\resources\swscale-8.dll" "$INSTDIR\swscale-8.dll"
  CopyFiles /SILENT "$INSTDIR\resources\postproc-58.dll" "$INSTDIR\postproc-58.dll"
!macroend

; Службу VPN регистрирует установщик (права администратора запрашиваются один раз, при установке).
; Всё пишется в %ProgramData%\OCHISTKA\install.log (строки установщика — латиницей: запись NSIS идёт в ANSI). Если регистрация не удалась, установка
; продолжается, а в программе режим «Весь компьютер» просит переустановку. Системные программы —
; по абсолютным путям, без поиска в PATH.
!include "x64.nsh"

!macro OCHISTKA_LOG TEXT
  ReadEnvStr $R8 ProgramData
  CreateDirectory "$R8\OCHISTKA"
  FileOpen $R9 "$R8\OCHISTKA\install.log" a
  FileSeek $R9 0 END
  FileWrite $R9 "${TEXT}$\r$\n"
  FileClose $R9
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro OCHISTKA_LOG "Installer: program folder $INSTDIR"
  ${DisableX64FSRedirection}
  nsExec::ExecToLog '"$WINDIR\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\vpn-service\vpn-service.ps1" -Action install -AppDir "$INSTDIR"'
  Pop $0
  ${EnableX64FSRedirection}
  !insertmacro OCHISTKA_LOG "Installer: VPN service registration finished with code $0"
  ${If} $0 != 0
    DetailPrint "Служба VPN не установлена, код $0 (подробности: install.log)"
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro OCHISTKA_LOG "Uninstall: program folder $INSTDIR"
  ${DisableX64FSRedirection}
  nsExec::ExecToLog '"$WINDIR\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\vpn-service\vpn-service.ps1" -Action uninstall -AppDir "$INSTDIR"'
  Pop $0
  ${EnableX64FSRedirection}
  !insertmacro OCHISTKA_LOG "Uninstall: VPN service removal finished with code $0"
!macroend

; Службу обхода регистрирует установщик (права администратора запрашиваются один раз, при установке).
; Те же команды выполняет проверка в CI (скрипт bypass-service.ps1). Системные программы — по
; абсолютным путям, без поиска в PATH.
!include "x64.nsh"

!macro NSIS_HOOK_POSTINSTALL
  ${DisableX64FSRedirection}
  nsExec::ExecToLog '"$WINDIR\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\ochistkadpi\bypass-service.ps1" -Action install -InstallDir "$INSTDIR\ochistkadpi"'
  Pop $0
  ${EnableX64FSRedirection}
  ${If} $0 != 0
    DetailPrint "Служба обхода не установлена, код $0"
    MessageBox MB_OK|MB_ICONEXCLAMATION "Не удалось установить службу обхода. Остальные возможности программы работают." /SD IDOK
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ${DisableX64FSRedirection}
  nsExec::ExecToLog '"$WINDIR\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\ochistkadpi\bypass-service.ps1" -Action uninstall -InstallDir "$INSTDIR\ochistkadpi"'
  Pop $0
  ${EnableX64FSRedirection}
!macroend

<#
  Скачивает VPN-ядро sing-box (официальный выпуск, без изменений) и проверяет SHA-256.
  Запуск из корня репозитория:
    powershell -ExecutionPolicy Bypass -File tools\vpn-core\fetch.ps1
  Файлы попадают в apps\desktop\src-tauri\resources\vpn-core\ (в git не хранятся) и
  упаковываются в приложение как ресурс Tauri. Версия и хэши зафиксированы ниже.
#>
$ErrorActionPreference = 'Stop'
$Version = '1.14.4'
$Url = "https://github.com/SagerNet/sing-box/releases/download/v$Version/sing-box-$Version-windows-amd64.zip"
$ArchiveHash = '99185d75d314a07dbba4a130c8ad986cbeb629a07927975636b341ca61f991df'
$Files = @{
  'sing-box.exe'   = '8f30682ee198855440d2a7fe070d5d3653793b2d01bb8c85eea88f93bbe7c8b9'
  'libcronet.dll'  = '3217c6260fbca5f16072e0b79735742f40109a63bb0ff88fd6b96dd6b54a2928'
  'LICENSE'        = 'bb3805862b583aee73ad6f7805ec634747a37257a637a3069857843f05ea589c'
}

$Root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
$Target = Join-Path $Root 'apps\desktop\src-tauri\resources\vpn-core'
New-Item -ItemType Directory -Force -Path $Target | Out-Null

function Test-Installed {
  foreach ($name in $Files.Keys) {
    $path = Join-Path $Target $name
    if (-not (Test-Path $path)) { return $false }
    if ((Get-FileHash $path -Algorithm SHA256).Hash.ToLower() -ne $Files[$name]) { return $false }
  }
  return $true
}

if (Test-Installed) { Write-Host "VPN-ядро sing-box $Version уже установлено, хэши совпали."; exit 0 }

$Work = Join-Path ([System.IO.Path]::GetTempPath()) ("ochistka-vpn-core-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $Work | Out-Null
try {
  $Zip = Join-Path $Work 'core.zip'
  Write-Host "Скачиваем $Url"
  [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
  Invoke-WebRequest -Uri $Url -OutFile $Zip -UseBasicParsing
  $actual = (Get-FileHash $Zip -Algorithm SHA256).Hash.ToLower()
  if ($actual -ne $ArchiveHash) { throw "SHA-256 архива не совпал: $actual (ожидали $ArchiveHash). Файл не установлен." }
  $Out = Join-Path $Work 'out'
  Expand-Archive -Path $Zip -DestinationPath $Out
  foreach ($name in $Files.Keys) {
    $found = Get-ChildItem -Path $Out -Recurse -File -Filter $name | Select-Object -First 1
    if (-not $found) { throw "В архиве нет файла $name" }
    $hash = (Get-FileHash $found.FullName -Algorithm SHA256).Hash.ToLower()
    if ($hash -ne $Files[$name]) { throw "SHA-256 файла $name не совпал: $hash. Файлы не установлены." }
  }
  foreach ($name in $Files.Keys) {
    $found = Get-ChildItem -Path $Out -Recurse -File -Filter $name | Select-Object -First 1
    Copy-Item -Force $found.FullName (Join-Path $Target $name)
  }
  Write-Host "Готово: sing-box $Version установлен в $Target"
}
finally {
  Remove-Item -Recurse -Force $Work -ErrorAction SilentlyContinue
}

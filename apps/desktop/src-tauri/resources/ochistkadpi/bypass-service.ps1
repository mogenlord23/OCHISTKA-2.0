# Служба «OchistkaBypass»: установка и удаление. Один и тот же файл использует установщик (от имени
# администратора, один раз) и проверка в CI. Службу запускает и останавливает обычный пользователь
# через права на саму службу (SDDL ниже), без запросов и окон. Автозапуска при старте Windows нет.
param(
    [Parameter(Mandatory = $true)][ValidateSet('install', 'uninstall')][string]$Action,
    [Parameter(Mandatory = $true)][string]$InstallDir
)
$ErrorActionPreference = 'Stop'
$ServiceName = 'OchistkaBypass'
$DriverName = 'WinDivert'
$Sc = Join-Path $env:SystemRoot 'System32\sc.exe'
# Права: система и администраторы — всё; интерактивные и сервисные учётные записи — чтение;
# «Прошедшие проверку пользователи» (AU) — только запрос состояния, запуск и остановка ЭТОЙ службы.
$Sddl = 'D:(A;;CCLCSWRPWPDTLOCRRC;;;SY)(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)(A;;CCLCSWLOCRRC;;;IU)(A;;CCLCSWLOCRRC;;;SU)(A;;LCRPWPLO;;;AU)S:(AU;FA;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;WD)'

function Get-State([string]$name) {
    $out = & $Sc query $name 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) { return 'NOT_INSTALLED' }
    if ($out -match 'STATE\s*:\s*\d+\s+(\w+)') { return $Matches[1] }
    return 'UNKNOWN'
}

function Wait-State([string]$name, [string]$state, [int]$seconds) {
    for ($i = 0; $i -lt ($seconds * 4); $i++) {
        if ((Get-State $name) -eq $state) { return $true }
        Start-Sleep -Milliseconds 250
    }
    return $false
}

function Remove-Service([string]$name) {
    if ((Get-State $name) -eq 'NOT_INSTALLED') { return }
    & $Sc stop $name | Out-Host
    [void](Wait-State $name 'STOPPED' 20)
    & $Sc delete $name | Out-Host
}

if ($Action -eq 'install') {
    $host_exe = Join-Path $InstallDir 'ochistka-bypass-host.exe'
    foreach ($f in @('ochistka-bypass-host.exe', 'ochistkadpi.exe', 'WinDivert.dll', 'WinDivert64.sys')) {
        if (-not (Test-Path (Join-Path $InstallDir $f))) { throw "Не найден файл: $f" }
    }
    Remove-Service $ServiceName
    New-Service -Name $ServiceName -BinaryPathName ('"{0}"' -f $host_exe) -DisplayName 'OCHISTKA Bypass' -StartupType Manual | Out-Null
    & $Sc description $ServiceName 'Обход сайтов (ochistkadpi). Запускается кнопкой в OCHISTKA.' | Out-Host
    & $Sc sdset $ServiceName $Sddl | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "sc sdset завершился с кодом $LASTEXITCODE" }
    & $Sc qc $ServiceName | Out-Host
    Write-Host "Служба $ServiceName установлена (запуск вручную)."
}
else {
    Remove-Service $ServiceName
    # Служба драйвера создаётся самим движком; убираем её, только если файл драйвера из нашей папки.
    $key = "HKLM:\SYSTEM\CurrentControlSet\Services\$DriverName"
    if (Test-Path $key) {
        $image = (Get-ItemProperty $key -ErrorAction SilentlyContinue).ImagePath
        if ($image -and $image.ToString().ToLowerInvariant().Contains($InstallDir.TrimEnd('\').ToLowerInvariant())) {
            & $Sc stop $DriverName | Out-Host
            [void](Wait-State $DriverName 'STOPPED' 10)
            & $Sc delete $DriverName | Out-Host
        }
    }
    Write-Host "Служба $ServiceName удалена."
}

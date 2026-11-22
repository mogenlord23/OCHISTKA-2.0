# Служба «OchistkaVpn»: установка и удаление. Один и тот же файл использует установщик (от имени
# администратора, один раз) и проверка в CI. Службу запускает и останавливает обычный пользователь
# через права на саму службу (SDDL ниже), без запросов и окон. Автозапуска при старте Windows нет.
# Каждый шаг пишется в журнал %ProgramData%\OCHISTKA\install.log; при ошибке — понятная причина.
param(
    [Parameter(Mandatory = $true)][ValidateSet('install', 'uninstall')][string]$Action,
    [Parameter(Mandatory = $true)][string]$AppDir
)
$ErrorActionPreference = 'Stop'
$ServiceName = 'OchistkaVpn'
$LogDir = Join-Path $env:ProgramData 'OCHISTKA'
$LogFile = Join-Path $LogDir 'install.log'
$Sc = Join-Path $env:SystemRoot 'System32\sc.exe'
# Права: система и администраторы — всё; интерактивные и сервисные учётные записи — чтение;
# «Прошедшие проверку пользователи» (AU) — только запрос состояния, запуск и остановка ЭТОЙ службы.
$Sddl = 'D:(A;;CCLCSWRPWPDTLOCRRC;;;SY)(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)(A;;CCLCSWLOCRRC;;;IU)(A;;CCLCSWLOCRRC;;;SU)(A;;LCRPWPLO;;;AU)S:(AU;FA;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;WD)'

function Write-Log([string]$message) {
    $line = '{0:yyyy-MM-dd HH:mm:ss}  {1}' -f (Get-Date), $message
    try {
        New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
        Add-Content -Path $LogFile -Value $line -Encoding UTF8
    } catch { }
    Write-Host $line
}

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

function Remove-ServiceIfAny([string]$name) {
    if ((Get-State $name) -eq 'NOT_INSTALLED') { return }
    Write-Log "Останавливаем и удаляем прежнюю службу $name"
    & $Sc stop $name 2>&1 | ForEach-Object { Write-Log "sc stop: $_" }
    [void](Wait-State $name 'STOPPED' 20)
    & $Sc delete $name 2>&1 | ForEach-Object { Write-Log "sc delete: $_" }
}

try {
    Write-Log "=== $Action, папка программы: $AppDir, пользователь: $([Security.Principal.WindowsIdentity]::GetCurrent().Name) ==="
    $isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    Write-Log "Права администратора: $isAdmin"
    if (-not $isAdmin) { throw 'Нет прав администратора для установки службы' }
    $hostExe = Join-Path $AppDir 'vpn-service\ochistka-vpn-host.exe'
    if ($Action -eq 'install') {
        foreach ($rel in @('vpn-service\ochistka-vpn-host.exe', 'vpn-core\sing-box.exe')) {
            $path = Join-Path $AppDir $rel
            $found = Test-Path $path
            Write-Log "Файл $rel : $(if ($found) { 'есть, ' + (Get-Item $path).Length + ' байт' } else { 'НЕ НАЙДЕН' })"
            if (-not $found) { throw "Не найден файл: $rel" }
        }
        Remove-ServiceIfAny $ServiceName
        New-Service -Name $ServiceName -BinaryPathName ('"{0}"' -f $hostExe) -DisplayName 'OCHISTKA VPN' -StartupType Manual | Out-Null
        Write-Log 'Служба создана (запуск вручную)'
        & $Sc description $ServiceName 'VPN для режима «Весь компьютер». Запускается кнопкой в OCHISTKA.' 2>&1 | ForEach-Object { Write-Log "sc description: $_" }
        & $Sc sdset $ServiceName $Sddl 2>&1 | ForEach-Object { Write-Log "sc sdset: $_" }
        if ($LASTEXITCODE -ne 0) { throw "sc sdset завершился с кодом $LASTEXITCODE" }
        & $Sc qc $ServiceName 2>&1 | ForEach-Object { Write-Log "sc qc: $_" }
        Write-Log "Служба $ServiceName установлена."
    }
    else {
        Remove-ServiceIfAny $ServiceName
        $run = Join-Path $AppDir 'vpn-service\run'
        if (Test-Path $run) { Remove-Item -Recurse -Force $run -ErrorAction SilentlyContinue }
        Write-Log "Служба $ServiceName удалена."
    }
    exit 0
}
catch {
    Write-Log "ОШИБКА: $($_.Exception.Message)"
    Write-Log ($_ | Out-String)
    exit 1
}

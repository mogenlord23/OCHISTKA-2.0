# OCHISTKA: безопасная проверка настоящего VPN-подключения на Windows.
# Скрипт ничего не меняет: он только читает системные настройки прокси и список процессов sing-box
# до подключения, во время него и после отключения. Подписки, ключи и адреса (в том числе адрес
# прокси) не читаются из приложения и не печатаются. Запуск (из папки OCHISTKA):
#   powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\vpn-check-windows.ps1

$ErrorActionPreference = 'Stop'
try { [Console]::OutputEncoding = [Text.Encoding]::UTF8 } catch {}
$key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings'
$backupKey = 'HKCU:\Software\OCHISTKA\Backup\VpnProxy'

function Read-Settings {
    $p = Get-ItemProperty -Path $key
    $server = if ($p.ProxyServer) { [string]$p.ProxyServer } else { '' }
    [pscustomobject]@{
        Enable = $(if ($null -ne $p.ProxyEnable) { [int]$p.ProxyEnable } else { 0 })
        Server = $server
        Pac    = $(if ($p.AutoConfigURL) { [string]$p.AutoConfigURL } else { '' })
        Cores  = @(Get-Process -Name 'sing-box' -ErrorAction SilentlyContinue).Count
        Backup = [bool](Test-Path $backupKey)
    }
}

# Адрес не печатаем: только «задан / не задан» и является ли он локальным.
function Show-State($title, $s) {
    $addr = if ($s.Server -eq '') { 'не задан' } elseif ($s.Server -match '^127\.0\.0\.1:\d+$') { 'задан (локальный, 127.0.0.1)' } else { 'задан (не локальный)' }
    $pac = if ($s.Pac -eq '') { 'не задан' } else { 'задан' }
    Write-Host $title
    Write-Host ('  Системный прокси включён: {0}; адрес прокси: {1}; сценарий автонастройки: {2}' -f $s.Enable, $addr, $pac)
    Write-Host ('  Процессов sing-box: {0}; резервная копия настроек OCHISTKA: {1}' -f $s.Cores, $(if ($s.Backup) { 'есть' } else { 'нет' }))
}

Write-Host ''
Write-Host 'Подготовка. Закройте OCHISTKA и любые другие VPN или прокси-программы.'
[void](Read-Host 'Когда закрыли, нажмите Enter')
$clean = Read-Settings
Show-State 'Исходное состояние (приложения закрыты):' $clean

$problems = @()
if ($clean.Enable -eq 1 -and $clean.Server -ne '') { $problems += 'В Windows уже включён системный прокси.' }
if ($clean.Pac -ne '') { $problems += 'В Windows задан сценарий автонастройки прокси.' }
if ($clean.Cores -gt 0) { $problems += ('Уже запущено процессов sing-box: {0}.' -f $clean.Cores) }
if ($clean.Backup) { $problems += 'От прошлого запуска OCHISTKA осталась резервная копия настроек прокси.' }
if ($problems.Count -gt 0) {
    Write-Host ''
    Write-Host 'ЧИСТЫЙ ТЕСТ НЕ ВЫПОЛНЯЕТСЯ: до теста уже что-то включено.'
    foreach ($m in $problems) { Write-Host ('  - ' + $m) }
    Write-Host ''
    Write-Host 'Что сделать (скрипт ничего не отключает сам, чтобы не трогать чужой VPN):'
    Write-Host '  1. Если это остатки прошлого запуска OCHISTKA: запустите OCHISTKA (npm run dev) и закройте её'
    Write-Host '     кнопкой закрытия окна. При запуске и выходе она сама уберёт свой прокси и свой sing-box.'
    Write-Host '  2. Если это другой VPN или программа: отключите её сами и повторите этот скрипт.'
    Write-Host '  3. Затем снова запустите этот скрипт.'
    Write-Host ''
    Write-Host 'Готово. Перешлите разработчику только этот текст: в нём нет ссылок и ключей.'
    exit 2
}

Write-Host ''
Write-Host 'Шаг 1. Запустите OCHISTKA (npm run dev), импортируйте подписку и выберите сервер.'
Write-Host '       VPN пока НЕ подключайте.'
[void](Read-Host 'Когда готово, нажмите Enter')
$before = Read-Settings
Show-State 'ДО подключения:' $before
if ($before.Enable -eq 1 -or $before.Cores -gt 0) {
    Write-Host '  ВНИМАНИЕ: до подключения уже что-то включено. Это не чистый тест: результат ниже не считать подтверждением.'
}

Write-Host ''
Write-Host 'Шаг 2. В OCHISTKA нажмите кнопку подключения и дождитесь результата на экране.'
[void](Read-Host 'Когда результат появился, нажмите Enter')
$during = Read-Settings
Show-State 'ВО ВРЕМЯ подключения:' $during
if ($during.Enable -eq 1 -and $during.Server -match '^127\.0\.0\.1:\d+$') {
    $proxy = 'http://' + $during.Server
    try {
        $r = Invoke-WebRequest -Uri 'https://www.gstatic.com/generate_204' -Proxy $proxy -UseBasicParsing -TimeoutSec 15
        Write-Host ('  Проверка трафика через прокси: ответ {0} (данные проходят)' -f $r.StatusCode)
    } catch {
        Write-Host ('  Проверка трафика через прокси: НЕ прошла (тип ошибки: {0})' -f $_.Exception.GetType().Name)
    }
} else {
    Write-Host '  Локальный прокси не включён: подключение не выполнено, закончилось ошибкой или OCHISTKA отказалась подключаться из-за чужого прокси.'
}

Write-Host ''
Write-Host 'Шаг 3. В OCHISTKA нажмите кнопку отключения. Если появится сообщение об ошибке, не закрывайте его.'
[void](Read-Host 'Когда отключилось, нажмите Enter')
$after = Read-Settings
Show-State 'ПОСЛЕ отключения:' $after

Write-Host ''
$restored = ($after.Enable -eq $before.Enable) -and ($after.Server -eq $before.Server) -and ($after.Pac -eq $before.Pac)
Write-Host ('Прежние настройки прокси возвращены: {0}' -f $(if ($restored) { 'ДА' } else { 'НЕТ' }))
Write-Host ('Осталось процессов sing-box: {0} (должно быть 0)' -f $after.Cores)
Write-Host ('Резервная копия настроек удалена: {0}' -f $(if ($after.Backup) { 'НЕТ (осталась)' } else { 'ДА' }))
Write-Host ''
Write-Host 'Готово. Перешлите разработчику только этот текст: в нём нет ссылок, ключей и адресов.'
